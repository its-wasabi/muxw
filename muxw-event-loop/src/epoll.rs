use std::os::unix::io::RawFd;

pub type Result<T> = std::result::Result<T, std::io::Error>;

fn ok_or_get_error(result: libc::c_int) -> Result<libc::c_int> {
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Interest: u32 {
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

impl Default for Interest {
    fn default() -> Self {
        Self::empty()
    }
}

impl Interest {
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Events: u32 {
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

impl Default for Events {
    fn default() -> Self {
        Self::empty()
    }
}

impl Events {
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

#[repr(i32)]
enum CtlOperation {
    Add = libc::EPOLL_CTL_ADD,
    Mod = libc::EPOLL_CTL_MOD,
    Del = libc::EPOLL_CTL_DEL,
}

#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Event {
    // TODO: Rename config to be more descriptive or create type that will describe it
    config: u32,
    data: u64,
}

impl Event {
    #[inline]
    #[must_use]
    pub const fn blank() -> Self {
        Self { config: 0, data: 0 }
    }

    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn fd(&self) -> RawFd {
        self.data as RawFd
    }

    #[inline]
    #[must_use]
    pub const fn events(self) -> Events {
        Events::from_bits_truncate(self.config)
    }
}

/// # Errors
/// Returns error when epoll failed to create
/// The fallout lies on the system side not caller
#[allow(dead_code)]
pub fn create(cloexec: bool) -> Result<RawFd> {
    let flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
    unsafe { ok_or_get_error(libc::epoll_create1(flags)) }
}

/// # Errors
/// Returns `Err(io::Error)` when:
/// - You provided invalid `epoll_fd`
/// - You provided invalid listener fd
#[allow(dead_code)]
pub fn add_fd(epoll_fd: RawFd, fd: std::os::fd::RawFd, interest: Interest) -> Result<()> {
    ctl(epoll_fd, CtlOperation::Add, fd, interest)
}

/// # Errors
/// Returns `Err(io::Error)` when:
/// - You provided invalid `epoll_fd`
/// - You provided invalid listener fd
#[allow(dead_code)]
pub fn mod_fd(epoll_fd: RawFd, fd: RawFd, interest: Interest) -> Result<()> {
    ctl(epoll_fd, CtlOperation::Mod, fd, interest)
}

/// # Errors
/// Returns `Err(io::Error)` when:
/// - You provided invalid `epoll_fd`
/// - You provided invalid listener fd
#[allow(dead_code)]
pub fn del_fd(epoll_fd: RawFd, fd: RawFd) -> Result<()> {
    ctl(epoll_fd, CtlOperation::Del, fd, Interest::empty())
}

#[allow(dead_code)]
fn ctl(epoll_fd: RawFd, operation: CtlOperation, fd: RawFd, interest: Interest) -> Result<()> {
    let mut config = libc::epoll_event {
        events: interest.bits(),
        #[allow(clippy::cast_sign_loss)]
        u64: fd as u64,
    };
    unsafe {
        ok_or_get_error(libc::epoll_ctl(
            epoll_fd,
            operation as i32,
            fd,
            &raw mut config,
        ))?
    };
    Ok(())
}

/// # Errors
/// Returns `Err(io::Error)` when:
/// - You provided invalid `epoll_fd`
///
/// # Panics
///
pub fn wait(epoll_fd: RawFd, timeout: Option<i32>, buf: &mut [Event]) -> Result<usize> {
    let timeout = timeout.unwrap_or(-1);

    let sys_buf = unsafe {
        std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<libc::epoll_event>(), buf.len())
    };

    let sys_buf_len = i32::try_from(sys_buf.len()).unwrap_or(i32::MAX);

    #[allow(clippy::expect_used)]
    let n = unsafe {
        usize::try_from(ok_or_get_error(libc::epoll_wait(
            epoll_fd,
            sys_buf.as_mut_ptr(),
            sys_buf_len,
            timeout,
        ))?)
        // TODO: create some ok_or_get_error which returns usize instead of c_int
        .unwrap_or_else(|_| unreachable!("ok_or_get_error guarantees value is >= 0"))
    };

    Ok(n)
}

#[allow(clippy::missing_errors_doc)]
pub fn close(epoll_fd: RawFd) -> Result<()> {
    ok_or_get_error(unsafe { libc::close(epoll_fd) })?;
    Ok(())
}
