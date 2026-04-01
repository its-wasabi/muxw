use std::os::unix::io::RawFd;

mod error;
mod event;
mod options;

pub enum Event {
    Input(RawFd),
    Config(Box<dyn muxw_types::config::ConfigEvent>),
    Wayland(RawFd),
}

pub struct EventLoop {
    epoll_fd: RawFd,
    events: std::collections::VecDeque<Box<Event>>,
}

impl EventLoop {
    pub fn new(cloexec: bool) -> error::Result<Self> {
        let flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
        let epoll_fd = unsafe { error::ok_or_get_error(libc::epoll_create1(flags))? };
        Ok(Self {
            epoll_fd,
            events: std::collections::VecDeque::new(),
        })
    }

    pub fn register(&mut self, event: E) -> error::Result<()> {
        ctl(
            self.epoll_fd,
            CtlOperation::Add,
            event.fd(),
            event.options(),
        );
        self.event_token_map.insert(event.fd() as u64, event);
        Ok(())
    }

    pub fn deregister(&mut self, event: E) -> error::Result<()> {
        self.event_token_map.remove(&(event.fd() as u64));
        ctl(
            self.epoll_fd,
            CtlOperation::Del,
            event.fd(),
            options::Options::empty(),
        )
    }

    pub fn querry(&mut self) -> error::Result<()> {
        Ok(())
    }

    pub fn wait() -> error::Result<()> {
        // TODO: If events present in buffer just return, if no simply call wait on epoll (to
        // wait & fetch new events)
        Ok(())
    }

    pub fn try_wait() -> Option<error::Result<()>> {
        Some(Ok(()))
    }
}

#[repr(i32)]
enum CtlOperation {
    Add = libc::EPOLL_CTL_ADD,
    Mod = libc::EPOLL_CTL_MOD,
    Del = libc::EPOLL_CTL_DEL,
}

#[allow(dead_code)]
fn ctl(
    epoll_fd: RawFd,
    operation: CtlOperation,
    fd: RawFd,
    interest: options::Options,
) -> error::Result<()> {
    let mut config = libc::epoll_event {
        events: interest.bits(),
        #[allow(clippy::cast_sign_loss)]
        u64: fd as u64,
    };
    unsafe {
        error::ok_or_get_error(libc::epoll_ctl(
            epoll_fd,
            operation as i32,
            fd,
            &raw mut config,
        ))?
    };
    Ok(())
}

#[repr(C)]
#[cfg_attr(target_arch = "x86_64", repr(packed))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EpollCallData {
    event_flags: u32,
    event_token: u64,
}

impl EpollCallData {
    #[inline]
    #[must_use]
    pub const fn blank() -> Self {
        Self {
            event_flags: 0,
            event_token: 0,
        }
    }

    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn fd(&self) -> RawFd {
        self.event_token as RawFd
    }
}

pub fn wait(
    epoll_fd: RawFd,
    timeout: Option<i32>,
    buf: &mut [EpollCallData],
) -> error::Result<usize> {
    let timeout = timeout.unwrap_or(-1);

    let sys_buf = unsafe {
        std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<libc::epoll_event>(), buf.len())
    };

    let sys_buf_len = i32::try_from(sys_buf.len()).unwrap_or(i32::MAX);

    #[allow(clippy::expect_used)]
    let n = unsafe {
        usize::try_from(error::ok_or_get_error(libc::epoll_wait(
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

impl Drop for EventLoop {
    fn drop(&mut self) {
        error::ok_or_get_error(unsafe { libc::close(self.epoll_fd) });
    }
}
