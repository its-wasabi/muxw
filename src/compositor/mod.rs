pub struct Compositor {
    wayland: crate::wayland::Wayland,
    drm: crate::backend::drm::Drm,
    renderer: crate::renderer::Renderer,
    input: crate::backend::input::Input,
    config: crate::config::Handle,

    // NOTE: Must be dropped after other components since it handles device close calls
    seat: crate::backend::seat::Seat,

    event_loop: crate::event_loop::EventLoop<crate::token::Token>,
    triggered_events: Vec<crate::token::Token>,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();
        let config = crate::config::Config::spawn(context, &mut event_loop)?;

        let seat = crate::backend::seat::Seat::new(&mut event_loop)?;
        let input = crate::backend::input::Input::new(&mut event_loop, seat.handle())?;

        let renderer = crate::renderer::Renderer::new()?;
        let drm = crate::backend::drm::Drm::new(seat.handle(), &mut event_loop)?;

        let wayland = crate::wayland::Wayland::new(context, &mut event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,
            config,

            seat,
            input,
            renderer,
            drm,

            wayland,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in self.triggered_events.drain(..) {
                let _span_guard = tracing::debug_span!("event", ?token).entered();

                match token {
                    crate::token::Token::Seat(event) => match event {
                        crate::backend::seat::Event::Dispatch => {
                            if let Err(error) = self.seat.dispatch() {
                                tracing::error!(?error, "Failed to dispatch libseat seat");
                            }
                        }

                        crate::backend::seat::Event::Enable => {
                            if let Err(error) = self.input.resume() {
                                tracing::error!(?error, "Failed to resume libinput");
                            }
                            if let Err(error) = self.drm.resume() {
                                tracing::error!(?error, "Failed to resume DRM");
                            }
                        }

                        crate::backend::seat::Event::Disable => {
                            self.input.suspend();
                            self.drm.suspend()?;

                            if let Err(error) = self.seat.disable() {
                                tracing::error!(?error, "Failed to ack Seat Disable");
                            }
                        }
                    },

                    crate::token::Token::Input(event) => match event {
                        crate::backend::input::Event::Dispatch => {
                            self.input.dispatch()?;
                        }

                        crate::backend::input::Event::Device { key, kind } => {
                            if let crate::backend::input::EventKind::Keyboard {
                                keycode,
                                state: input::event::keyboard::KeyState::Pressed,
                            } = kind
                            {
                                tracing::trace!("KEY PRESS: {keycode:?}");

                                let esc = u32::from(evdev::KeyCode::KEY_ESC.code());
                                let f1 = u32::from(evdev::KeyCode::KEY_F1.code());
                                let f12 = u32::from(evdev::KeyCode::KEY_F12.code());

                                if keycode == esc {
                                    self.event_loop.emitter().emit(crate::token::Token::Exit);
                                } else if (f1..=f12).contains(&keycode) {
                                    // (XKB_KEY_XF86Switch_VT_1 .. 12)
                                    let vt = (keycode - f1 + 1) as i32;
                                    tracing::info!(vt, "Requesting VT switch");
                                    if let Err(error) = self.seat.switch_session(vt) {
                                        tracing::error!(?error, vt, "Failed to switch VT session");
                                    }
                                }
                            }
                        }
                    },

                    crate::token::Token::Drm(event) => {
                        self.drm.dispatch(event, &mut self.event_loop)?;
                    }

                    crate::token::Token::Wayland(event) => {
                        self.wayland.dispatch(event, &self.event_loop)?;
                    }

                    crate::token::Token::Config(command) => {
                        eprintln!("Apply config logic here");
                    }

                    crate::token::Token::Exit => {
                        tracing::trace!("EXIT {:?}", std::time::Instant::now());

                        return Ok(());
                    }
                }
            }
        }
    }
}
