pub struct HandlerEntry {
    _source: Box<dyn std::any::Any>,
    callback: Box<dyn FnMut() -> bool>,
}

pub struct EventLoop {
    epoll_fd: std::os::fd::RawFd,
    event_buf: Vec<crate::Event>,
    handlers: std::collections::HashMap<std::os::fd::RawFd, HandlerEntry>,
}

impl EventLoop {
    pub fn new() -> crate::Result<Self> {
        Ok(Self {
            epoll_fd: crate::epoll::create(true)?,
            event_buf: Vec::new(),
            handlers: std::collections::HashMap::new(),
        })
    }

    pub fn register<S, C>(&mut self, source: S, mut callback: C) -> crate::Result<()>
    where
        S: crate::sources::EventSource + 'static,
        C: FnMut(S::InnerType) -> bool + 'static,
    {
        let fd = source.fd()?;
        let interest = source.interest();
        crate::epoll::add_fd(self.epoll_fd, fd, interest)?;

        // We need to call source.read_event() inside the closure but we're
        // also moving source into _source (Box<dyn Any>). The trick: store a
        // raw pointer to source's location *inside* the box, which is stable
        // once boxed. Then the closure calls through that pointer.
        //
        // Safety contract: _source and read_fn live in the same HandlerEntry
        // and are dropped together — the pointer is always valid when called.
        let boxed: Box<S> = Box::new(source);
        let src_ptr: *const S = &*boxed;

        self.handlers.insert(
            fd,
            HandlerEntry {
                _source: boxed,
                callback: Box::new(move || {
                    let event = unsafe { (*src_ptr).read_event() };
                    callback(event)
                }),
            },
        );

        Ok(())
    }

    pub fn deregister(&mut self, fd: std::os::fd::RawFd) -> std::io::Result<()> {
        crate::epoll::del_fd(self.epoll_fd, fd)?;
        self.handlers.remove(&fd);
        Ok(())
    }

    pub fn dispatch(&mut self) -> std::io::Result<()> {
        let n = crate::epoll::wait(self.epoll_fd, None, &mut self.event_buf)?;

        let ready: Vec<std::os::fd::RawFd> = self.event_buf[..n]
            .iter()
            .map(super::epoll::Event::fd)
            .collect();

        let mut to_remove = vec![];
        for fd in ready {
            if let Some(entry) = self.handlers.get_mut(&fd) {
                if (entry.callback)() {
                    to_remove.push(fd);
                }
            }
        }

        for fd in to_remove {
            let _ = self.deregister(fd);
        }

        Ok(())
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        // TODO: handle error or log it at the least
        let _ = crate::epoll::close(self.epoll_fd);
    }
}
