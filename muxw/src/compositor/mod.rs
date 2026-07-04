#![feature(associated_type_defaults)]

mod event_loop;

use std::os::fd::{AsFd, AsRawFd};

const DISPATCH_BATCH_CAPACITY: std::num::NonZero<usize> = std::num::NonZero::new(32).unwrap();

#[derive(Debug, Clone, Copy)]
enum Token {}

pub struct Compositor {
    event_loop: event_loop::EventLoop<Token>,
    // display: wayland_server::Display<Self>,
    // config_command: std::rc::Rc<crate::config::CommandHandle>,
}

impl Compositor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let event_loop = event_loop::EventLoop::new()?;

        // let (config_command, config_event_rx) =
        //     crate::config::Config::spawn(&crate::PATH.config_file)?;
        //
        // event_loop.register_source(crate::event_loop::Source::Config(config_event_rx));
        //
        // config_command.send(muxw_types::config::ConfigCommand::KeyboardAdded { device: () });
        // config_command.send(muxw_types::config::ConfigCommand::KeyboardInactive(
        //     std::time::Duration::from_millis(1000),
        // ));
        //
        // let mut display =
        //     wayland_server::Display::new().map_err(|err| crate::error::InitError::Wayland {
        //         action: "create display",
        //         source: err,
        //     })?;
        //
        // let wayland_event_fd = display.backend().poll_fd().as_raw_fd();
        // event_loop.register(crate::event_loop::Source::Wayland(wayland_event_fd));

        Ok(Self {
            event_loop,
            // display,
            // config_command,
        })
    }
}

struct CompositorState {
    workspaces: Vec<()>,
}

struct EventLoop {}

trait EventSource {
    type Data;
    fn fd(&self) -> std::os::fd::RawFd;
    fn callback(&mut self, state: &mut Self::Data);
}

impl EventLoop {
    fn dispatch(&mut self) {
        // Instead of returning events we call the callback() function of the trait and pass state
        // to it
    }

    fn run(&mut self) {
        self.dispatch()
    }
}
