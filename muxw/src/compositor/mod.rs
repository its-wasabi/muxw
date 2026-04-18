use std::os::fd::{AsFd, AsRawFd};

pub struct Compositor {
    display: wayland_server::Display<Self>,
    config_command: std::rc::Rc<crate::config::CommandHandle>,
}

impl Compositor {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<Self, crate::error::InitError> {
        let (config_command, config_event_rx) =
            crate::config::Config::spawn(&crate::PATH.config_file)?;
        event_loop.register(crate::event_loop::Source::Config(config_event_rx));

        config_command.send(muxw_types::config::ConfigCommand::KeyboardAdded);

        let mut display =
            wayland_server::Display::new().map_err(|err| crate::error::InitError::Wayland {
                action: "create display",
                source: err,
            })?;
        let wayland_event_fd = display.backend().poll_fd().as_raw_fd();
        event_loop.register(crate::event_loop::Source::Wayland(wayland_event_fd));

        Ok(Self {
            display,
            config_command,
        })
    }
}
