// TODO: Clean that up

const DEFAULT_CAPACITY: usize = 16;

slotmap::new_key_type! {
    struct TimerKey;
}

pub enum TimerMode {
    Exact(std::time::Instant),
    Delay(std::time::Duration),
    Rapid(std::time::Duration),
}

impl TimerMode {
    fn deadline(&self, now: std::time::Instant) -> std::time::Instant {
        match *self {
            Self::Exact(instant) => instant,
            Self::Delay(duration) | Self::Rapid(duration) => now + duration,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Deadline {
    at: std::time::Instant,
    key: TimerKey,
}

impl Deadline {
    #[inline]
    fn is_expired(&self, now: std::time::Instant) -> bool {
        self.at <= now
    }
}

struct Entry<T> {
    mode: TimerMode,
    token: T,
}

pub(super) struct Timers<T> {
    deadlines: std::collections::BinaryHeap<std::cmp::Reverse<Deadline>>,
    registry: std::rc::Rc<std::cell::RefCell<slotmap::SlotMap<TimerKey, Entry<T>>>>,
}

impl<T> Timers<T> {
    pub(super) fn new() -> Self {
        let map = slotmap::SlotMap::with_capacity_and_key(DEFAULT_CAPACITY);
        let registry = std::cell::RefCell::new(map);
        Self {
            deadlines: std::collections::BinaryHeap::new(),
            registry: std::rc::Rc::new(registry),
        }
    }

    pub(super) fn timeout(&mut self) -> Option<std::time::Duration> {
        if self.deadlines.is_empty() {
            return None;
        }

        let registry = self.registry.borrow();
        while let Some(&std::cmp::Reverse(deadline)) = self.deadlines.peek() {
            if registry.contains_key(deadline.key) {
                let now = std::time::Instant::now();
                return Some(deadline.at.saturating_duration_since(now));
            }

            // TODO: Make that work with entry to binary_heap rather than not logically connected
            // data
            self.deadlines.pop();
        }

        None
    }

    fn register(&mut self, entry: Entry<T>) -> TimerKey {
        let now = std::time::Instant::now();
        let at = entry.mode.deadline(now);
        let key = self.registry.borrow_mut().insert(entry);
        let deadline = Deadline { at, key };
        self.deadlines.push(std::cmp::Reverse(deadline));
        key
    }
}

#[must_use = "dropping a Handle immediately cancels the timer"]
pub struct Handle<T> {
    key: TimerKey,
    registry: std::rc::Rc<std::cell::RefCell<slotmap::SlotMap<TimerKey, Entry<T>>>>,
}

impl<T> Handle<T> {
    pub fn cancel(&self) {
        self.registry.borrow_mut().remove(self.key);
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl<T: Clone> super::EventLoop<T> {
    pub fn register_timer(&mut self, mode: TimerMode, token: impl Into<T>) -> Handle<T> {
        let token = token.into();
        let entry = Entry { mode, token };
        let key = self.timers.register(entry);

        Handle {
            key,
            registry: self.timers.registry.clone(),
        }
    }

    pub(super) fn drain_ready_timers_into(&mut self, triggered: &mut Vec<T>) {
        if self.timers.deadlines.is_empty() {
            return;
        }

        let now = std::time::Instant::now();
        let registry = self.timers.registry.borrow_mut();

        while let Some(mut top) = self.timers.deadlines.peek_mut() {
            let std::cmp::Reverse(deadline) = *top;
            if !deadline.is_expired(now) {
                break;
            }

            let Some(entry) = registry.get(deadline.key) else {
                std::collections::binary_heap::PeekMut::pop(top);
                continue;
            };

            triggered.push(entry.token.clone());

            if let TimerMode::Rapid(period) = entry.mode {
                let step = period.max(std::time::Duration::from_nanos(1));
                top.0.at += step;
                if top.0.at <= now {
                    top.0.at = now + step;
                }
            } else {
                std::collections::binary_heap::PeekMut::pop(top);
            }
        }
    }
}
