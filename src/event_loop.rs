const SOURCES_DEFAULT_CAPACITY: usize = 128;
const TIMERS_DEFAULT_CAPACITY: usize = 32;
const EMITERS_DEFAULT_CAPACITY: usize = 32;

#[derive(Debug, Clone, Copy)]
struct TokenEntry<T> {
    token: T,
    kind: EntryKind,
}

impl<T> TokenEntry<T> {
    const fn new_io(token: T, mode: polling::PollMode) -> Self {
        let kind = EntryKind::Io(mode);
        Self { token, kind }
    }

    const fn new_timer(token: T, mode: TimerMode) -> Self {
        let kind = EntryKind::Timer(mode);
        Self { token, kind }
    }

    const fn is_oneshot(&self) -> bool {
        match self.kind {
            EntryKind::Io(poll_mode) => matches!(
                poll_mode,
                polling::PollMode::Oneshot | polling::PollMode::EdgeOneshot
            ),
            EntryKind::Timer(timer_mode) => {
                matches!(timer_mode, TimerMode::Delay(_) | TimerMode::Periodic(_))
            }

            EntryKind::Dead => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Io(polling::PollMode),
    Timer(TimerMode),
    Dead,
}

pub struct EventLoop<T: Clone> {
    poller: std::sync::Arc<polling::Poller>,

    emits: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<T>>>,
    channel: crossbeam_channel::Receiver<T>,
    timers: std::collections::BinaryHeap<Timer>,
    sources: slab::Slab<TokenEntry<T>>,

    sender: crossbeam_channel::Sender<T>,
    events: polling::Events,
}

impl<T: Clone> EventLoop<T> {
    pub fn new() -> std::io::Result<Self> {
        let poller = std::sync::Arc::new(polling::Poller::new()?);
        let emits = std::rc::Rc::new(std::cell::RefCell::new(
            std::collections::VecDeque::with_capacity(EMITERS_DEFAULT_CAPACITY),
        ));
        let (sender, channel) = crossbeam_channel::unbounded();
        let timers = std::collections::BinaryHeap::with_capacity(TIMERS_DEFAULT_CAPACITY);
        let sources = slab::Slab::with_capacity(SOURCES_DEFAULT_CAPACITY);
        let events = polling::Events::new();

        Ok(Self {
            poller,

            emits,
            channel,
            timers,
            sources,

            sender,
            events,
        })
    }

    pub fn dispatch(&mut self, triggered: &mut Vec<T>) -> std::io::Result<()> {
        triggered.clear();

        // 1. Emitter
        triggered.extend(self.emits.borrow_mut().drain(..));

        // 2. Channel
        while let Ok(token) = self.channel.try_recv() {
            triggered.push(token);
        }

        self.events.clear();

        let timeout = if triggered.is_empty() {
            self.timers.peek().map(Timer::since_now)
        } else {
            Some(std::time::Duration::ZERO)
        };
        self.poller.wait(&mut self.events, timeout)?;

        // 3. Timer
        let now = std::time::Instant::now();
        while let Some(mut top_timer) = self.timers.peek_mut() {
            if !top_timer.is_expired(now) {
                break;
            }

            let Some(entry) = self.sources.get(top_timer.key.get()) else {
                std::collections::binary_heap::PeekMut::pop(top_timer);
                continue;
            };

            if matches!(entry.kind, EntryKind::Dead) {
                self.sources.remove(top_timer.key.get());
                std::collections::binary_heap::PeekMut::pop(top_timer);
                continue;
            }

            triggered.push(entry.token.clone());
            if let EntryKind::Timer(TimerMode::Periodic(duration)) = entry.kind {
                top_timer.deadline = now + duration;
            } else {
                self.sources.remove(top_timer.key.get());
                std::collections::binary_heap::PeekMut::pop(top_timer);
            }
        }

        // 4. Sources
        let sources = &mut self.sources;
        triggered.extend(self.events.iter().filter_map(|event| {
            let entry = sources.get(event.key)?;
            if entry.is_oneshot() {
                Some(sources.remove(event.key).token)
            } else {
                Some(entry.token.clone())
            }
        }));

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IoKey(usize);

impl IoKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

impl<T: Clone> EventLoop<T> {
    pub fn register_source(
        &mut self,
        source: impl polling::AsRawSource + polling::AsSource,
        mode: polling::PollMode,
        token: T,
    ) -> std::io::Result<IoKey> {
        let key = self.sources.insert(TokenEntry::new_io(token, mode));
        let interest = polling::Event::readable(key);
        unsafe { self.poller.add_with_mode(source, interest, mode)? };
        Ok(IoKey(key))
    }

    pub fn deregister_source(
        &mut self,
        key: IoKey,
        source: impl polling::AsSource,
    ) -> std::io::Result<()> {
        if self.sources.try_remove(key.get()).is_none() {
            return Ok(());
        }

        self.poller
            .delete(source)
            .or_else(|error| match error.kind() {
                // TODO: Think if you should leave only NotFound ErrorKind
                std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => Ok(()),
                _ => Err(error),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerKey(usize);

impl TimerKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TimerMode {
    Delay(std::time::Duration),
    Exact(std::time::Instant),
    Periodic(std::time::Duration),
}

impl TimerMode {
    fn deadline(&self) -> std::time::Instant {
        match self {
            Self::Exact(instant) => *instant,
            Self::Delay(duration) | Self::Periodic(duration) => {
                std::time::Instant::now() + *duration
            }
        }
    }

    fn make_timer(self, key: TimerKey) -> Timer {
        let deadline = self.deadline();
        Timer::new(key, deadline)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Timer {
    key: TimerKey,
    deadline: std::time::Instant,
}

impl Timer {
    pub const fn new(key: TimerKey, deadline: std::time::Instant) -> Self {
        Self { key, deadline }
    }

    pub fn is_expired(&self, at: std::time::Instant) -> bool {
        self.deadline <= at
    }

    pub fn since_now(&self) -> std::time::Duration {
        self.deadline
            .saturating_duration_since(std::time::Instant::now())
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline.cmp(&self.deadline)
    }
}

impl<T: Clone> EventLoop<T> {
    pub fn register_timer(&mut self, mode: TimerMode, token: T) -> TimerKey {
        let key = TimerKey(self.sources.insert(TokenEntry::new_timer(token, mode)));
        let timer = mode.make_timer(key);
        self.timers.push(timer);
        key
    }

    pub fn lazy_deregister_timer(&mut self, key: TimerKey) {
        if let Some(entry) = self.sources.get_mut(key.get()) {
            entry.kind = EntryKind::Dead;
        }
    }

    pub fn deregister_timer(&mut self, key: TimerKey) {
        self.sources.try_remove(key.get());
        self.timers.retain(|timer| timer.key != key);
    }
}

#[derive(Clone)]
pub struct EventSender<T> {
    poller: std::sync::Arc<polling::Poller>,
    sender: crossbeam_channel::Sender<T>,
}

impl<T: Clone> EventSender<T> {
    const fn new(
        poller: std::sync::Arc<polling::Poller>,
        sender: crossbeam_channel::Sender<T>,
    ) -> Self {
        Self { poller, sender }
    }

    pub fn send(&self, token: T) {
        self.sender.send(token);
        self.poller.notify();
    }
}

impl<T: Clone> EventLoop<T> {
    pub fn sender(&self) -> EventSender<T> {
        EventSender::new(self.poller.clone(), self.sender.clone())
    }
}

#[derive(Clone)]
pub struct EventEmitter<T> {
    emits: std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<T>>>,
}

impl<T: Clone> EventEmitter<T> {
    fn new(emits: &std::rc::Rc<std::cell::RefCell<std::collections::VecDeque<T>>>) -> Self {
        Self {
            emits: std::rc::Rc::clone(emits),
        }
    }

    pub fn emit(&self, token: T) {
        self.emits.borrow_mut().push_back(token);
    }
}

impl<T: Clone> EventLoop<T> {
    pub fn emiter(&self) -> EventEmitter<T> {
        EventEmitter::new(&self.emits)
    }
}
