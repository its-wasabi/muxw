const DEFAULT_CAPACITY: usize = 32;

slotmap::new_key_type! {
    struct IoKey;
}

impl TryFrom<IoKey> for usize {
    type Error = std::num::TryFromIntError;
    fn try_from(value: IoKey) -> Result<Self, Self::Error> {
        usize::try_from(value.0.as_ffi())
    }
}

impl From<usize> for IoKey {
    #[inline]
    fn from(value: usize) -> Self {
        slotmap::KeyData::from_ffi(value as u64).into()
    }
}

#[derive(Clone, Copy)]
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
        if let Err(error) = self.sources.poller.delete(fd)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::error!(?error, "failed to delete io source from poller");
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
        let raw_key: usize = io_key
            .try_into()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let interest = match interest {
            Interest::Readable => polling::Event::readable(raw_key),
            Interest::Writable => polling::Event::writable(raw_key),
            Interest::Both => polling::Event::all(raw_key),
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
