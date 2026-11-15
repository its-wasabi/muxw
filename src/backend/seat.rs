pub struct Seat {
    handle: crate::event_loop::IoHandle<crate::token::Token>,
    libseat: std::ptr::NonNull<libseat_sys::libseat>,
    emitter: *mut crate::event_loop::Emitter<crate::token::Token>,
}

impl Seat {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        static LISTENER: libseat_sys::libseat_seat_listener = libseat_sys::libseat_seat_listener {
            enable_seat: Some(on_enable_seat),
            disable_seat: Some(on_disable_seat),
        };

        let emitter = Box::into_raw(Box::new(event_loop.emitter()));
        let seatlib_ptr =
            unsafe { libseat_sys::libseat_open_seat(&raw const LISTENER, emitter.cast()) };
        let Some(libseat) = std::ptr::NonNull::new(seatlib_ptr) else {
            drop(unsafe { Box::from_raw(emitter) });
            return Err(Box::new(std::io::Error::last_os_error()));
        };

        let raw_fd = unsafe { libseat_sys::libseat_get_fd(libseat.as_ptr()) };
        if raw_fd < 0 {
            unsafe { libseat_sys::libseat_close_seat(libseat.as_ptr()) };
            drop(unsafe { Box::from_raw(emitter) });
            return Err(Box::new(std::io::Error::last_os_error()));
        }

        let fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) };

        let handle = match event_loop.register_fd(
            &fd,
            crate::event_loop::Interest::Readable,
            crate::token::Token::Seat(Event::Dispatch),
        ) {
            Ok(handle) => handle,
            Err(err) => {
                unsafe { libseat_sys::libseat_close_seat(libseat.as_ptr()) };
                drop(unsafe { Box::from_raw(emitter) });
                return Err(Box::new(err));
            }
        };

        Ok(Self {
            handle,
            libseat,
            emitter,
        })
    }

    pub fn dispatch(&self) -> std::io::Result<()> {
        let ret = unsafe { libseat_sys::libseat_dispatch(self.libseat.as_ptr(), 0) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn disable(&self) -> std::io::Result<()> {
        let ret = unsafe { libseat_sys::libseat_disable_seat(self.libseat.as_ptr()) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn name(&self) -> &str {
        let ptr = unsafe { libseat_sys::libseat_seat_name(self.libseat.as_ptr()) };
        unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap_or("seat0") }
    }

    pub fn switch_session(&self, vt: i32) -> std::io::Result<()> {
        let ret = unsafe { libseat_sys::libseat_switch_session(self.libseat.as_ptr(), vt) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[inline]
    pub const fn handle(&self) -> SeatHandle {
        SeatHandle(self.libseat)
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        unsafe { libseat_sys::libseat_close_seat(self.libseat.as_ptr()) };
        std::mem::drop(unsafe { Box::from_raw(self.emitter) });
    }
}

#[derive(Clone, Copy)]
pub struct SeatHandle(std::ptr::NonNull<libseat_sys::libseat>);

impl SeatHandle {
    pub fn open_device(self, path: &std::path::Path) -> std::io::Result<Device> {
        use std::os::fd::FromRawFd;
        use std::os::unix::ffi::OsStrExt;

        let mut raw_fd = std::os::fd::RawFd::default();
        let path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null byte")
        })?;

        let id = unsafe {
            libseat_sys::libseat_open_device(self.0.as_ptr(), path.as_ptr(), &raw mut raw_fd)
        };
        if id < 0 || raw_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let id = DeviceId(id);
        let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(raw_fd) };

        Ok(Device { fd, id })
    }

    pub fn open_device_raw(self, path: &std::path::Path) -> Result<Device, i32> {
        self.open_device(path).map_err(|error| {
            error
                .raw_os_error()
                .unwrap_or_else(|| rustix::io::Errno::IO.raw_os_error())
        })
    }

    pub fn close_device(&self, id: DeviceId) -> std::io::Result<()> {
        let ret = unsafe { libseat_sys::libseat_close_device(self.0.as_ptr(), id.0) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

pub struct Device {
    pub fd: std::os::fd::OwnedFd,
    pub id: DeviceId,
}

#[derive(Clone, Copy)]
pub struct DeviceId(std::ffi::c_int);

unsafe extern "C" fn on_enable_seat(
    _seat: *mut libseat_sys::libseat,
    emitter: *mut std::ffi::c_void,
) {
    let emitter = unsafe { &*(emitter as *const crate::event_loop::Emitter<crate::token::Token>) };
    emitter.emit(crate::token::Token::Seat(Event::Enable));
}

unsafe extern "C" fn on_disable_seat(
    _seat: *mut libseat_sys::libseat,
    emitter: *mut std::ffi::c_void,
) {
    let emitter = unsafe { &*(emitter as *const crate::event_loop::Emitter<crate::token::Token>) };
    emitter.emit(crate::token::Token::Seat(Event::Disable));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Dispatch,
    Enable,
    Disable,
}
