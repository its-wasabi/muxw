const DEFAULT_DEVICES_CAPACITY: usize = 2;

pub struct SeatManager {
    seat: libseat::Seat,
    seat_devices: std::collections::HashMap<std::os::unix::io::RawFd, libseat::Device>,
}

impl SeatManager {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let emitter = event_loop.emiter();
        let mut seat = libseat::Seat::open(move |_, seat_event| match seat_event {
            libseat::SeatEvent::Enable => emitter.emit(crate::token::Token::SeatEnable),
            libseat::SeatEvent::Disable => emitter.emit(crate::token::Token::SeatDisable),
        })?;

        event_loop.register_source(
            &seat.get_fd()?,
            polling::PollMode::Edge,
            crate::token::Token::SeatEvent,
        )?;

        let seat_devices = std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY);

        Ok(Self { seat, seat_devices })
    }

    pub fn dispatch(&mut self) -> std::io::Result<()> {
        self.seat
            .dispatch(0)
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))
            .map(|_| ())
    }

    pub fn disable(&mut self) -> std::io::Result<()> {
        self.seat
            .disable()
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))
    }

    pub fn open_device(
        &mut self,
        open_data: Box<crate::token::SeatOpenData>,
    ) -> std::io::Result<()> {
        let _span_guard = tracing::info_span!("seat device open", path = ?open_data.path).entered();

        match self.seat.open_device(&open_data.path) {
            Ok(device) => {
                use std::os::fd::{AsFd, AsRawFd};

                match rustix::io::dup(device.as_fd()) {
                    Ok(owned_fd) => {
                        let raw_fd = owned_fd.as_raw_fd();
                        self.seat_devices.insert(raw_fd, device);

                        let _ = open_data.reply.send(Ok(owned_fd));

                        Ok(())
                    }
                    Err(rustix_err) => {
                        let errno = rustix_err.raw_os_error();

                        tracing::error!(errno = errno, "Failed to duplicate device FD");

                        if let Err(errno) = self.seat.close_device(device) {
                            tracing::error!("Failed to close libseat device after dup error");
                        }

                        let _ = open_data
                            .reply
                            .send(Err(std::io::Error::from_raw_os_error(errno.into())));

                        Err(std::io::Error::from_raw_os_error(errno))
                    }
                }
            }
            Err(errno) => {
                tracing::error!("libseat failed to open device");

                let _ = open_data
                    .reply
                    .send(Err(std::io::Error::from_raw_os_error(errno.into())));

                Err(std::io::Error::from_raw_os_error(errno.into()))
            }
        }
    }

    pub fn close_device(
        &mut self,
        raw_fd: std::os::fd::RawFd,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let device = self.seat_devices.remove(&raw_fd).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Device FD not found in seat manager",
            )
        })?;

        self.seat
            .close_device(device)
            .map_err(|errno| std::io::Error::from_raw_os_error(errno.into()))?;

        Ok(())
    }
}
