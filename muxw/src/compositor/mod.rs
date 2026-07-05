pub mod input;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::Token>,
    triggered_events: Vec<crate::Token>,

    config_command_sender: crossbeam_channel::Sender<crate::config::ConfigCommand>,
    input_manager: input::InputManager,
}

impl Compositor {
    pub fn new(context: &crate::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let config_command_sender =
            crate::config::Config::spawn(&context.path.config_file, event_loop.channel_sender());

        let input_manager = input::InputManager::new(event_loop.channel_sender())?;

        Ok(Self {
            event_loop,
            triggered_events,

            config_command_sender,
            input_manager,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in &self.triggered_events {
                match token {
                    crate::Token::Input(event) => self.input_manager.process_event(event),
                    crate::Token::Config(command) => println!("EV::CONFIG::{command:#?}"),
                }
            }
        }
    }
}
