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
