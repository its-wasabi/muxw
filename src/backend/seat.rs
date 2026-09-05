const DEFAULT_DEVICES_CAPACITY: usize = 12;

struct Inner {
    seat: libseat::Seat,
    devices: std::collections::HashMap<std::os::unix::io::RawFd, libseat::Device>,
}

pub struct SeatManager(std::rc::Rc<std::cell::RefCell<Inner>>);

impl SeatManager {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let emitter = event_loop.emiter();
        let mut seat = libseat::Seat::open(move |_, seat_event| match seat_event {
            libseat::SeatEvent::Enable => {
                emitter.emit(crate::token::Token::Seat(SeatEvent::Enable))
            }
            libseat::SeatEvent::Disable => {
                emitter.emit(crate::token::Token::Seat(SeatEvent::Disable))
            }
        })?;

        event_loop.register_source(
            &seat.get_fd()?,
            polling::PollMode::Edge,
            crate::token::Token::Seat(SeatEvent::Dispatch),
        );

        let inner = Inner {
            seat,
            devices: std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY),
        };

        Ok(Self(std::rc::Rc::new(std::cell::RefCell::new(inner))))
    }

    pub fn dispatch(&self) -> std::io::Result<()> {
        self.0
            .borrow_mut()
            .seat
            .dispatch(0)
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))
            .map(|_| ())
    }

    pub fn disable(&self) -> std::io::Result<()> {
        self.0
            .borrow_mut()
            .seat
            .disable()
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))
    }
}

#[derive(Clone)]
pub struct SeatHandle(std::rc::Rc<std::cell::RefCell<Inner>>);

impl SeatHandle {
    pub fn open_device(&self, path: &std::path::Path) -> std::io::Result<std::os::fd::OwnedFd> {
        use std::os::fd::{AsFd, AsRawFd};
        let _span_guard = tracing::info_span!("seat device open", ?path).entered();
        let mut inner = self.0.borrow_mut();

        let device = inner
            .seat
            .open_device(&path)
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))?;

        match rustix::io::dup(device.as_fd()) {
            Ok(owned_fd) => {
                inner.devices.insert(owned_fd.as_raw_fd(), device);
                Ok(owned_fd)
            }
            Err(rustix_err) => {
                let errno = rustix_err.raw_os_error();
                tracing::error!(errno, "Failed to duplicate device FD");
                if let Err(close_errno) = inner.seat.close_device(device) {
                    tracing::error!(
                        ?close_errno,
                        "Failed to close libseat device after dup error"
                    );
                }
                Err(std::io::Error::from_raw_os_error(errno))
            }
        }
    }

    pub fn close_device(&self, raw_fd: std::os::fd::RawFd) -> std::io::Result<()> {
        let mut inner = self.0.borrow_mut();
        let device = inner.devices.remove(&raw_fd).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Device FD not found in seat manager",
            )
        })?;

        inner
            .seat
            .close_device(device)
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))
    }
}

impl SeatManager {
    pub fn handle(&self) -> SeatHandle {
        SeatHandle(self.0.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SeatEvent {
    Dispatch,
    Enable,
    Disable,
}
