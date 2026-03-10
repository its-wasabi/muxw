pub trait EventSource {
    type InnerType;
    fn fd(&self) -> crate::Result<std::os::fd::RawFd>;
    fn interest(&self) -> crate::epoll::Interest {
        crate::epoll::Interest::default()
    }
    fn read_event(&self) -> Self::InnerType;
}

pub mod config;
