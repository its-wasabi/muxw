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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Options: u32 {
        /// Monitor for data available to read
        const READ = libc::EPOLLIN as u32;
        /// Monitor for ready to write without blocking
        const WRITE = libc::EPOLLOUT as u32;
        /// Monitor for urgent out-of-band data (TCP OOB data, rarely used)
        const URGENT = libc::EPOLLPRI as u32;
        /// Monitor for error conditions (always monitored automatically, but can be explicit)
        const ERROR = libc::EPOLLERR as u32;
        /// Monitor for hang up (always monitored automatically, but can be explicit)
        const HANG_UP = libc::EPOLLHUP as u32;
        /// Monitor for peer closing their write end (graceful shutdown detection)
        const CLOSED = libc::EPOLLRDHUP as u32;

        /// Use edge-triggered mode (only notify on state changes)
        const EDGE_TRIGGERED = libc::EPOLLET as u32;
        /// One-shot mode (automatically disable after one event)
        const ONE_SHOT = libc::EPOLLONESHOT as u32;
        /// Exclusive wakeup (wake only one epoll instance, not all)
        const EXCLUSIVE = libc::EPOLLEXCLUSIVE as u32;
        /// Prevent system suspend while handling events (requires CAP_BLOCK_SUSPEND)
        const WAKE_UP = libc::EPOLLWAKEUP as u32;
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::empty()
    }
}

impl Options {
    #[inline]
    #[must_use]
    pub fn readable(mut self) -> Self {
        self |= Self::READ;
        self
    }

    #[inline]
    #[must_use]
    pub fn writable(mut self) -> Self {
        self |= Self::WRITE;
        self
    }

    #[inline]
    #[must_use]
    pub fn urgent(mut self) -> Self {
        self |= Self::URGENT;
        self
    }

    #[inline]
    #[must_use]
    pub fn error(mut self) -> Self {
        self |= Self::ERROR;
        self
    }

    #[inline]
    #[must_use]
    pub fn hang_up(mut self) -> Self {
        self |= Self::HANG_UP;
        self
    }

    #[inline]
    #[must_use]
    pub fn closed(mut self) -> Self {
        self |= Self::CLOSED;
        self
    }

    #[inline]
    #[must_use]
    pub fn edge_triggered(mut self) -> Self {
        self |= Self::EDGE_TRIGGERED;
        self
    }

    #[inline]
    #[must_use]
    pub fn one_shot(mut self) -> Self {
        self |= Self::ONE_SHOT;
        self
    }

    #[inline]
    #[must_use]
    pub fn exclusive(mut self) -> Self {
        self |= Self::EXCLUSIVE;
        self
    }

    #[inline]
    #[must_use]
    pub fn wake_up(mut self) -> Self {
        self |= Self::WAKE_UP;
        self
    }
}
