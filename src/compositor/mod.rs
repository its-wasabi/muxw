pub struct Compositor {
    event_loop: crate::event_loop::EventLoop,
    triggered_events: Vec<crate::token::Token>,
    config: crate::config::Handle,

    seat: crate::backend::seat::Seat,
    input: crate::backend::input::Input,
    renderer: crate::renderer::Renderer,
    drm: crate::backend::drm::Drm,

    wayland: crate::wayland::Wayland,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();
        let config = crate::config::Config::spawn(context, &mut event_loop)?;

        let seat_manager = crate::backend::seat::Seat::new(&mut event_loop)?;
        let input_manager =
            crate::backend::input::Input::new(&mut event_loop, seat_manager.handle())?;

        let renderer = crate::renderer::Renderer::new()?;
        let drm_manager = crate::backend::drm::Drm::new(&mut event_loop)?;

        let wayland_server = crate::wayland::Wayland::new(context, &mut event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,
            config,

            seat: seat_manager,
            input: input_manager,
            renderer,
            drm: drm_manager,

            wayland: wayland_server,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in self.triggered_events.drain(..) {
                let _span_guard = tracing::debug_span!("event", ?token).entered();

                match token {
                    crate::token::Token::Seat(event) => match event {
                        crate::backend::seat::SeatEvent::Dispatch => {
                            if let Err(error) = self.seat.dispatch() {
                                tracing::error!(?error, "Failed to dispatch libseat seat");
                            }
                        }

                        crate::backend::seat::SeatEvent::Enable => {
                            tracing::debug!("Seat Enable");
                            // if let Err(error) = self.drm_manager.resume() {
                            //     tracing::error!("Failed to restore DRM state: {error}");
                            // }
                        }

                        crate::backend::seat::SeatEvent::Disable => {
                            tracing::debug!("Seat Disable <- THE IMPORTANT ONE");
                            // self.drm_manager.pause();
                            self.drm.drop_master()?;
                        }
                    },

                    crate::token::Token::Input(event) => match event {
                        crate::backend::input::InputEvent::Dispatch => {
                            self.input.dispatch()?;
                        }

                        crate::backend::input::InputEvent::Device { key: _, kind } => {
                            tracing::debug!(?event, "Input event received");

                            let esc = evdev::KeyCode::KEY_ESC.code();
                            let _space = evdev::KeyCode::KEY_SPACE.code();

                            if let crate::backend::input::EventKind::Keyboard {
                                keycode,
                                state: input::event::keyboard::KeyState::Pressed,
                            } = kind
                                && keycode == u32::from(esc)
                            {
                                self.event_loop.emiter().emit(crate::token::Token::Exit);
                            }
                        }
                    },

                    crate::token::Token::Drm(event) => {
                        tracing::trace!("Drm Event triggered");
                        self.drm.dispatch(&mut self.event_loop, event)?;
                    }

                    crate::token::Token::Wayland(event) => {
                        self.wayland.dispatch(event, &self.event_loop)?;
                    }

                    crate::token::Token::Config(command) => {
                        tracing::debug!(?command, "Config command received");
                        eprintln!("Apply config logic here");
                        self.event_loop.emiter().emit(crate::token::Token::Exit);
                    }

                    crate::token::Token::Exit => {
                        tracing::debug!("Exiting");

                        if let Err(error) = self.seat.disable() {
                            tracing::error!(?error, "Failed to ack Seat Disable during Shutdown");
                        }

                        return Ok(());
                    }
                }
            }
        }
    }
}
