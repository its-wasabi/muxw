use std::os::unix::io::RawFd;

pub trait EventSource {
    fn fd(&self) -> RawFd;
    fn options(&self) -> crate::options::Options;
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct EventFlags: u32 {
        /// Data is available to read
        const READABLE = libc::EPOLLIN as u32;
        /// Can write without blocking
        const WRITABLE = libc::EPOLLOUT as u32;
       /// Urgent out-of-band data available
        const URGENT = libc::EPOLLPRI as u32;

        /// Error condition occurred
        const ERROR = libc::EPOLLERR as u32;
        /// Hang up (connection closed)
        const HANG_UP = libc::EPOLLHUP as u32;
        /// Peer closed their write end
        const READ_CLOSED = libc::EPOLLRDHUP as u32;
    }
}

impl Default for EventFlags {
    fn default() -> Self {
        Self::empty()
    }
}

impl EventFlags {
    /// Check if the fd is readable
    #[inline]
    #[must_use]
    pub const fn is_readable(self) -> bool {
        self.contains(Self::READABLE)
    }

    /// Check if the fd is writable
    #[inline]
    #[must_use]
    pub const fn is_writable(self) -> bool {
        self.contains(Self::WRITABLE)
    }

    /// Check if there's urgent data
    #[inline]
    #[must_use]
    pub const fn is_urgent(self) -> bool {
        self.contains(Self::URGENT)
    }

    /// Check if there was an error
    #[inline]
    #[must_use]
    pub const fn is_error(self) -> bool {
        self.contains(Self::ERROR)
    }

    /// Check if the connection hung up
    #[inline]
    #[must_use]
    pub const fn is_hang_up(self) -> bool {
        self.contains(Self::HANG_UP)
    }

    /// Check if the peer closed their side
    #[inline]
    #[must_use]
    pub const fn is_read_closed(self) -> bool {
        self.contains(Self::READ_CLOSED)
    }

    /// Check if the connection is closed (either hang up or read closed)
    #[inline]
    #[must_use]
    pub fn is_closed(self) -> bool {
        self.intersects(Self::HANG_UP | Self::READ_CLOSED)
    }
}

impl<T> EventSource for std::sync::mpsc::Receiver<T> {
    fn fd(&self) -> RawFd {
        todo!()
    }
    fn options(&self) -> crate::options::Options {
        todo!()
    }
}

impl EventSource for RawFd {
    fn fd(&self) -> RawFd {}

    fn options(&self) -> crate::options::Options {}
}
