use std::os::unix::io::RawFd;

mod error;
mod event;
mod sources;

pub use sources::Source;

const INIT_EVENT_BUFFER_CAPACITY: usize = 128;

pub struct EventLoop {
    epoll_fd: RawFd,
    // TODO: Use enum instead of Dispatch
    event_buf: Vec<Box<libc::epoll_event>>,
}

impl EventLoop {
    pub fn new(cloexec: bool) -> error::Result<Self> {
        let epoll_flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
        let epoll_fd = error::ok_or_get_error(unsafe { libc::epoll_create1(epoll_flags) })?;

        Ok(Self {
            epoll_fd,
            event_buf: Vec::with_capacity(INIT_EVENT_BUFFER_CAPACITY),
        })
    }

    pub fn register(&mut self, source: sources::Source) {}

    pub fn run(&mut self, state: &mut crate::compositor::Compositor) {
        // FIXME:
        here!("event loop start");
        loop {}
    }
}

struct EpollCallData {
    event_flags: u32,
    data: u64,
}

impl EpollCallData {}

#[repr(i32)]
enum CtlOperation {
    Add = libc::EPOLL_CTL_ADD,
    Mod = libc::EPOLL_CTL_MOD,
    Del = libc::EPOLL_CTL_DEL,
}

fn ctl(
    epoll_fd: RawFd,
    operation: CtlOperation,
    fd: RawFd,
    interest: event::Options,
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
