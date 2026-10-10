pub struct Compositor {
    event_loop: crate::event_loop::EventLoop,
    triggered_events: Vec<crate::token::Token>,

    seat_manager: crate::backend::seat::SeatManager,
    input_manager: crate::backend::input::InputManager,
    drm_manager: crate::backend::drm::DrmManager,

    wayland_server: crate::wayland::WaylandServer,

    renderer: crate::renderer::Renderer,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let seat_manager = crate::backend::seat::SeatManager::new(&mut event_loop)?;
        let input_manager =
            crate::backend::input::InputManager::new(&mut event_loop, seat_manager.handle())?;

        let renderer = crate::renderer::Renderer::new()?;
        let drm_manager = crate::backend::drm::DrmManager::new(&mut event_loop, &renderer)?;

        let wayland_server = crate::wayland::WaylandServer::new(context, &mut event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,

            seat_manager,
            input_manager,
            drm_manager,

            wayland_server,

            renderer,
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
                            if let Err(error) = self.seat_manager.dispatch() {
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
                            // self.drm_manager.drop_master();
                            // if let Err(error) = self.seat_manager.disable() {
                            //     tracing::error!(?error, "Failed to Disable Seat");
                            // }
                        }
                    },

                    crate::token::Token::Input(event) => match event {
                        crate::backend::input::InputEvent::Dispatch => {
                            self.input_manager.dispatch()?;
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

                    crate::token::Token::Drm(event) => match event {
                        crate::backend::drm::Event::Udev => {
                            tracing::trace!("DrmUdev monitor event triggered");
                            self.drm_manager
                                .dispatch_udev(&mut self.event_loop, &self.renderer);
                        }

                        crate::backend::drm::Event::Card(drm_card_key) => {
                            tracing::trace!(?drm_card_key, "DrmCard event triggered");
                            self.drm_manager.dispatch_card(
                                drm_card_key,
                                &self.renderer,
                                [0.0, 0.0, 0.0, 1.0],
                            );
                        }
                    },

                    crate::token::Token::Wayland(event) => {
                        self.wayland_server.dispatch(event, &mut self.event_loop);
                    }

                    crate::token::Token::Config(command) => {
                        tracing::debug!(?command, "Config command received");
                    }

                    crate::token::Token::Exit => {
                        tracing::debug!("Exiting");

                        if let Err(error) = self.seat_manager.disable() {
                            tracing::error!(?error, "Failed to ack Seat Disable during Shutdown");
                        }

                        return Ok(());
                    }
                }
            }
        }
    }
}
