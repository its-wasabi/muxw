mod input;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::Token>,
    triggered_events: Vec<crate::Token>,
    config_command_sender: crossbeam_channel::Sender<crate::config::ConfigCommand>,
    input_manager: input::InputManager,
}

impl Compositor {
    pub fn new(context: &crate::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::with_capacity(crate::event_loop::DISPATCH_BATCH_CAPACITY.get());

        let config_command_sender =
            crate::config::Config::spawn(&context.path.config_file, event_loop.channel_sender());

        let input_manager = input::InputManager::new()?;
        event_loop.register_source(
            &input_manager.fd(),
            polling::PollMode::Edge,
            crate::Token::Input(0),
        )?;

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

            config_command_sender,
            input_manager,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in &self.triggered_events {
                println!("Event: {token:?}");

                match token {
                    crate::Token::Input(_) => self.input_manager.dispatch()?,
                    crate::Token::Config(_) => println!("EV::CONFIG"),
                }
            }
        }
    }
}
