const SOURCES_DEFAULT_CAPACITY: usize = 128;
const TIMERS_DEFAULT_CAPACITY: usize = 32;

pub struct EventLoop<T: Clone> {
    poller: std::sync::Arc<polling::Poller>,
    sources: slab::Slab<TokenEntry<T>>,
    timers: std::collections::BinaryHeap<Timer>,
    channel: crossbeam_channel::Receiver<T>,

    sender: crossbeam_channel::Sender<T>,
    events: polling::Events,
}

impl<T: Clone> EventLoop<T> {
    pub fn new() -> std::io::Result<Self> {
        let (sender, channel) = crossbeam_channel::unbounded();
        Ok(Self {
            poller: std::sync::Arc::new(polling::Poller::new()?),
            sources: slab::Slab::with_capacity(SOURCES_DEFAULT_CAPACITY),
            timers: std::collections::BinaryHeap::with_capacity(TIMERS_DEFAULT_CAPACITY),
            channel,
            sender,

            events: polling::Events::new(),
        })
    }

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

    pub fn dispatch(&mut self, triggered: &mut Vec<T>) -> std::io::Result<()> {
        triggered.clear();
        self.events.clear();

        let timeout = self.timers.peek().map(Timer::since_now);
        self.poller.wait(&mut self.events, timeout)?;

        let sources = &mut self.sources;
        triggered.extend(self.events.iter().filter_map(|event| {
            let entry = sources.get(event.key)?;
            if entry.is_oneshot() {
                Some(sources.remove(event.key).token)
            } else {
                Some(entry.token.clone())
            }
        }));

        while let Ok(token) = self.channel.try_recv() {
            triggered.push(token);
        }

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

        Ok(())
    }

    pub fn channel_sender(&self) -> EventSender<T> {
        EventSender::new(self.poller.clone(), self.sender.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IoKey(usize);

impl IoKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimerKey(usize);

impl TimerKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

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

#[derive(Clone)]
pub struct EventSender<T> {
    sender: crossbeam_channel::Sender<T>,
    poller: std::sync::Arc<polling::Poller>,
}

impl<T: Clone> EventSender<T> {
    const fn new(
        poller: std::sync::Arc<polling::Poller>,
        sender: crossbeam_channel::Sender<T>,
    ) -> Self {
        Self { sender, poller }
    }

    pub fn send(&self, token: T) {
        self.sender.send(token);
        self.poller.notify();
    }
}
