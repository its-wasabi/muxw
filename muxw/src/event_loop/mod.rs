use std::{
    collections::HashMap,
    os::{fd::AsRawFd, unix::io::RawFd},
};
pub mod channel;
pub mod error;
pub mod event;
pub mod sources;
pub use sources::Source;

const INIT_EVENT_BUFFER_CAPACITY: usize = 8;

pub struct EventLoop {
    epoll_fd: RawFd,
    event_buf: Vec<libc::epoll_event>,
    // TODO: Try to find alternative without lookups
    sources: HashMap<RawFd, Source>,
}

impl EventLoop {
    pub fn new(cloexec: bool) -> error::Result<Self> {
        let epoll_flags = if cloexec { libc::EPOLL_CLOEXEC } else { 0 };
        let epoll_fd = error::ok_or_get_error(unsafe { libc::epoll_create1(epoll_flags) })?;
        Ok(Self {
            epoll_fd,
            event_buf: Vec::with_capacity(INIT_EVENT_BUFFER_CAPACITY),
            sources: HashMap::new(),
        })
    }

    pub fn register(&mut self, source: Source) {
        let fd = source.fd().as_raw_fd();
        ctl(self.epoll_fd, CtlOperation::Add, fd, source.options());
        self.sources.insert(fd, source);
    }

    // IMPORTANT: This block is vibe coded review it yourself as soon as possible
    pub fn run(&mut self, compositor: &mut crate::compositor::Compositor) {
        here!("event loop start");
        loop {
            let n = unsafe {
                self.event_buf.set_len(self.event_buf.capacity());
                let n = libc::epoll_wait(
                    self.epoll_fd,
                    self.event_buf.as_mut_ptr(),
                    self.event_buf.capacity() as i32,
                    -1,
                );
                if n < 0 {
                    if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                        self.event_buf.set_len(0);
                        continue;
                    }
                    here!("epoll_wait error: {}", std::io::Error::last_os_error());
                    self.event_buf.set_len(0);
                    break;
                }
                self.event_buf.set_len(n as usize);
                n as usize
            };

            let ready: Vec<(RawFd, event::EventFlags)> = self.event_buf[..n]
                .iter()
                .map(|e| {
                    (
                        e.u64 as RawFd,
                        event::EventFlags::from_bits_truncate(e.events),
                    )
                })
                .collect();

            for (fd, flags) in ready {
                if let Some(source) = self.sources.get(&fd) {
                    source.dispatch(compositor, flags);
                }
            }
        }
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
