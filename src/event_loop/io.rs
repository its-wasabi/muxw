use std::os::fd::AsRawFd;

slotmap::new_key_type! {
    struct IoKey;
}

impl Into<usize> for IoKey {
    fn into(self) -> usize {
        self.0.as_ffi() as usize
    }
}

impl From<usize> for IoKey {
    fn from(value: usize) -> Self {
        slotmap::KeyData::from_ffi(value as u64).into()
    }
}

pub enum Interest {
    Readable,
    Writable,
}

pub(super) struct Sources<T>(std::rc::Rc<std::cell::RefCell<slotmap::SlotMap<IoKey, T>>>);

impl<T> Sources<T> {
    pub(super) fn new() -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(
            slotmap::SlotMap::with_key(),
        )))
    }

    fn insert(&self, token: T) -> IoKey {
        self.0.borrow_mut().insert(token)
    }

    fn remove(&self, key: IoKey) {
        self.0.borrow_mut().remove(key);
    }
}

impl<T> Clone for Sources<T> {
    fn clone(&self) -> Self {
        Self(std::rc::Rc::clone(&self.0))
    }
}

pub struct Handle<T> {
    fd: std::os::fd::OwnedFd,
    key: IoKey,
    poller: std::sync::Arc<polling::Poller>,
    sources: Sources<T>,
}

impl<T> Handle<T> {
    fn cancel(&self) {
        self.sources.remove(self.key);
        if let Err(error) = self.poller.delete(&self.fd) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::error!(?error, "failed to delete io source from poller");
            }
        }
    }
}

impl<T: Clone> super::EventLoop<T> {
    pub fn register_io<S: std::os::fd::AsFd>(
        &mut self,
        source: S,
        interest: Interest,
        mode: polling::PollMode,
        token: impl Into<T>,
    ) -> Result<Handle<T>, std::io::Error> {
        let fd = source.as_fd().try_clone_to_owned()?;
        let key: IoKey = self.sources.insert(token.into());
        let interest = match interest {
            Interest::Readable => polling::Event::readable(key.into()),
            Interest::Writable => polling::Event::writable(key.into()),
        };

        if let Err(error) = unsafe { self.poller.add_with_mode(&fd, interest, mode) } {
            self.sources.remove(key);
            tracing::error!(?error, "failed to add io source to poller");
            return Err(error);
        }

        Ok(Handle {
            fd,
            key,
            poller: self.poller.clone(),
            sources: self.sources.clone(),
        })
    }

    pub(super) fn drain_ready_io_into(&self, triggered: &mut Vec<T>) {
        let sources = self.sources.0.borrow();
        triggered.reserve(self.events.len());
        triggered.extend(
            self.events
                .iter()
                .filter_map(|event| sources.get(event.key.into()).cloned()),
        );
    }
}
