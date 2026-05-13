use std::{
    collections::VecDeque,
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
    sync::{Arc, Mutex},
};

struct Inner<T> {
    efd: OwnedFd,
    queue: Mutex<VecDeque<T>>,
}

fn efd_create() -> io::Result<OwnedFd> {
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn efd_signal(fd: RawFd) {
    let val: u64 = 1u64;
    unsafe {
        libc::write(fd, &raw const val as *const libc::c_void, 8);
    }
}

fn efd_drain(fd: RawFd) -> io::Result<()> {
    let mut val: u64 = 0;
    let ret = unsafe { libc::read(fd, &raw mut val as *mut libc::c_void, 8) as i32 };
    if ret < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        {
            self.inner.queue.lock().unwrap().push_back(value);
        }
        efd_signal(self.inner.efd.as_raw_fd());
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

unsafe impl<T: Send> Send for Sender<T> {}
unsafe impl<T: Send> Sync for Sender<T> {}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    pub fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.efd.as_fd()
    }

    pub fn drain(&self, mut f: impl FnMut(T)) -> io::Result<()> {
        efd_drain(self.inner.efd.as_raw_fd())?;
        let items: VecDeque<T> = {
            let mut q = self.inner.queue.lock().unwrap();
            std::mem::take(&mut *q)
        };
        for item in items {
            f(item);
        }
        Ok(())
    }
}

unsafe impl<T: Send> Send for Receiver<T> {}

pub fn channel<T: Send>() -> io::Result<(Sender<T>, Receiver<T>)> {
    let inner = Arc::new(Inner {
        efd: efd_create()?,
        queue: Mutex::new(VecDeque::new()),
    });
    Ok((
        Sender {
            inner: Arc::clone(&inner),
        },
        Receiver { inner },
    ))
}
