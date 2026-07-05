use std::os::fd::{AsFd, AsRawFd};

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::Token>,
    triggered_events: Vec<crate::Token>,

    config_event_sender: crossbeam_channel::Sender<crate::config::ConfigEvent>, // display: wayland_server::Display<Self>,
}

impl Compositor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::with_capacity(crate::event_loop::DISPATCH_BATCH_CAPACITY.get());

        let config_event_sender = crate::config::Config::spawn(event_loop.channel_sender());

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
            triggered_events,

            config_event_sender,
            // display,
            // config_command,
        })
    }

    pub fn run(&mut self) {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events);

            for event in &self.triggered_events {
                println!("Event: {event:?}");
            }
        }
    }
}
