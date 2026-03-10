pub trait EventFd {
    fn fd(&self) -> std::os::fd::RawFd;
}

mod event_counter_fd;
pub use event_counter_fd::EventCounterFd;
