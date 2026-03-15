use std::os::fd::{AsRawFd, FromRawFd};

pub struct EventCounterFd(std::os::fd::OwnedFd);

impl super::EventFd for EventCounterFd {
    fn fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.0.as_raw_fd()
    }
}

impl EventCounterFd {
    pub fn new() -> std::io::Result<Self> {
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) }))
    }

    pub fn write(&self) {
        let val: u64 = 1;
        unsafe {
            libc::write(
                self.as_raw_fd(),
                &val as *const u64 as *const libc::c_void,
                8,
            )
        };
    }

    #[allow(clippy::must_use_candidate)]
    pub fn drain(&self) -> u64 {
        let mut val: u64 = 0;
        unsafe { libc::read(self.0.as_raw_fd(), &raw mut val as *mut libc::c_void, 8) };
        val
    }

    pub fn try_clone(&self) -> std::io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }
}

impl std::os::fd::AsRawFd for EventCounterFd {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.0.as_raw_fd()
    }
}
