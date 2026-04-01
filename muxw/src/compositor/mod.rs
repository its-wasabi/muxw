use std::os::fd::{AsFd, AsRawFd};

pub struct Muxw {
    event_loop: muxw_event_loop::EventLoop,
    display: wayland_server::Display<Self>,
}

impl Muxw {
    pub fn new() -> Result<Self, crate::error::InitError> {
        let mut event_loop = muxw_event_loop::EventLoop::new().unwrap();

        let (config_command, config_event) = crate::config::Config::new(&crate::PATH.config_file);
        event_loop.register(muxw_event_loop::Event::Config(config_event));

        let mut display = wayland_server::Display::new().unwrap();
        let wayland_event = display.backend().poll_fd().as_raw_fd();
        event_loop.register(muxw_event_loop::Event::Wayland(wayland_event));

        Ok(Self {
            event_loop,
            display,
        })
    }

    pub fn run(&mut self) {
        while let event = self.event_loop.dispatch() {
            todo!("Handle that event");
        }
    }
}
