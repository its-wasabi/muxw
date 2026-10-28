const DEFAULT_CAPACITY: usize = 32;

slotmap::new_key_type! {
    struct IoKey;
}

impl Into<usize> for IoKey {
    #[inline]
    fn into(self) -> usize {
        self.0.as_ffi() as usize
    }
}

impl From<usize> for IoKey {
    #[inline]
    fn from(value: usize) -> Self {
        slotmap::KeyData::from_ffi(value as u64).into()
    }
}

pub enum Interest {
    Readable,
    Writable,
    Both,
}

#[derive(Clone)]
pub(super) struct Sources<T> {
    poller: std::sync::Arc<polling::Poller>,
    tokens: std::cell::RefCell<slotmap::SlotMap<IoKey, T>>,
}

impl<T> Sources<T> {
    pub(super) fn new(poller: std::sync::Arc<polling::Poller>) -> Self {
        let map = slotmap::SlotMap::with_capacity_and_key(DEFAULT_CAPACITY);
        let tokens = std::cell::RefCell::new(map);
        Sources { poller, tokens }
    }

    #[inline]
    fn insert(&self, token: T) -> IoKey {
        self.tokens.borrow_mut().insert(token)
    }

    #[inline]
    fn remove(&self, key: IoKey) {
        self.tokens.borrow_mut().remove(key);
    }
}

#[must_use = "dropping a Handle immediately cancels the source"]
pub struct Handle<T> {
    raw_fd: std::os::fd::RawFd,
    io_key: IoKey,
    sources: std::rc::Rc<Sources<T>>,
}

impl<T> Handle<T> {
    fn cancel(&self) {
        self.sources.remove(self.io_key);
        let fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(self.raw_fd) };
        if let Err(error) = self.sources.poller.delete(fd) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::error!(?error, "failed to delete io source from poller");
            }
        }
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl<T: Clone> super::EventLoop<T> {
    pub fn register_fd<S: polling::AsRawSource>(
        &mut self,
        source: S,
        interest: Interest,
        token: impl Into<T>,
    ) -> Result<Handle<T>, std::io::Error> {
        use std::os::fd::AsRawFd;
        let raw_fd = source.raw().as_raw_fd();

        let mode = polling::PollMode::Edge;
        let io_key: IoKey = self.sources.insert(token.into());
        let interest = match interest {
            Interest::Readable => polling::Event::readable(io_key.into()),
            Interest::Writable => polling::Event::writable(io_key.into()),
            Interest::Both => polling::Event::all(io_key.into()),
        };

        if let Err(error) = unsafe { self.poller.add_with_mode(source, interest, mode) } {
            self.sources.remove(io_key);
            tracing::error!(?error, "failed to add io source to poller");
            return Err(error);
        }

        Ok(Handle {
            raw_fd,
            io_key,
            sources: self.sources.clone(),
        })
    }

    pub(super) fn drain_ready_io_into(&self, triggered: &mut Vec<T>) {
        if self.events.is_empty() {
            return;
        }

        let tokens = self.sources.tokens.borrow();
        triggered.reserve(self.events.len());
        triggered.extend(
            self.events
                .iter()
                .filter_map(|event| tokens.get(event.key.into()).cloned()),
        );
    }
}
