use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

fn create_eventfd() -> super::error::Result<OwnedFd> {
    Ok(unsafe {
        OwnedFd::from_raw_fd(super::error::ok_or_get_error(libc::eventfd(
            0,
            libc::EFD_CLOEXEC | libc::EFD_NONBLOCK,
        ))?)
    })
}

// TODO: Check if .cast() is necessary
pub(super) fn eventfd_signal(fd: RawFd) {
    let mut val: u64 = 1;
    unsafe { libc::write(fd, (&raw mut val).cast(), 8) };
}

pub(super) fn eventfd_drain(fd: RawFd) -> super::error::Result<()> {
    let mut val: u64 = 0;
    match super::error::ok_or_get_error(unsafe { libc::read(fd, (&raw mut val).cast(), 8) as i32 })
    {
        // TODO: Check what the fuck it wanted to do here
        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => Err(err),
        Err(err) => Err(err),
        Ok(_) => Ok(()),
    }
}

pub struct NotifySyncSender<T> {
    inner: std::sync::mpsc::SyncSender<T>,
    efd: std::rc::Rc<OwnedFd>,
}

pub struct NotifyReceiver<T> {
    pub(super) inner: std::sync::mpsc::Receiver<T>,
    efd: std::rc::Rc<OwnedFd>,
}

impl<T: Send> NotifySyncSender<T> {
    pub fn send(&self, msg: T) -> Result<(), std::sync::mpsc::SendError<T>> {
        self.inner.send(msg)?;
        eventfd_signal(self.efd.as_raw_fd());
        Ok(())
    }
}

impl<T> NotifyReceiver<T> {
    pub fn as_raw_fd(&self) -> RawFd {
        self.efd.as_raw_fd()
    }

    pub fn drain(&self, mut f: impl FnMut(T)) -> super::error::Result<()> {
        eventfd_drain(self.efd.as_raw_fd())?;
        while let Ok(msg) = self.inner.try_recv() {
            f(msg);
        }
        Ok(())
    }
}
