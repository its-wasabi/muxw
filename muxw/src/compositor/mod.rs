mod event_loop;

use std::os::fd::{AsFd, AsRawFd};

const DISPATCH_BATCH_CAPACITY: std::num::NonZero<usize> = std::num::NonZero::new(32).unwrap();

#[derive(Debug, Clone, Copy)]
enum Token {
    Timer(usize),
}

pub struct Compositor {
    event_loop: event_loop::EventLoop<Token>,
    triggered_events: Vec<Token>,
    // display: wayland_server::Display<Self>,
    // config_command: std::rc::Rc<crate::config::CommandHandle>,
}

impl Compositor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = event_loop::EventLoop::new()?;
        let triggered_events = Vec::with_capacity(DISPATCH_BATCH_CAPACITY.get());

        event_loop.register_timer(
            event_loop::TimerMode::Periodic(std::time::Duration::from_millis(200)),
            Token::Timer(200),
        );

        event_loop.register_timer(
            event_loop::TimerMode::Exact(
                std::time::Instant::now() + std::time::Duration::from_secs(2),
            ),
            Token::Timer(2000),
        );

        let x = event_loop.channel_sender();
        x.send(Token::Timer(1920));

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
            triggered_events,
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
