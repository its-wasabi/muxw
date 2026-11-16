mod seat;

pub struct Compositor {
    wayland: crate::wayland::Wayland,
    drm: crate::backend::drm::Drm,
    renderer: crate::renderer::Renderer,
    config: crate::config::Handle,

    input: crate::backend::input::Input,
    triggered_input: Vec<crate::backend::input::InputEvent>,

    event_loop: crate::event_loop::EventLoop<crate::token::Token>,
    triggered_events: Vec<crate::token::Token>,

    // NOTE: Must be dropped after other components since it handles device close calls
    seat: crate::backend::seat::Seat,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let config = crate::config::Config::spawn(context, &mut event_loop)?;
        let seat = crate::backend::seat::Seat::new(&mut event_loop)?;
        let renderer = crate::renderer::Renderer::new()?;
        let drm = crate::backend::drm::Drm::new(seat.handle(), &mut event_loop)?;

        let input = crate::backend::input::Input::new(&mut event_loop, seat.handle())?;
        let triggered_input = Vec::new();

        let wayland = crate::wayland::Wayland::new(context, &mut event_loop)?;

        Ok(Self {
            wayland,
            drm,
            renderer,
            config,

            input,
            triggered_input,

            event_loop,
            triggered_events,

            seat,
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

                    crate::token::Token::Input(crate::backend::input::Event::Dispatch) => {
                        self.input.dispatch(&mut self.triggered_input)?;
                        for event in &self.triggered_input {
                            if let crate::backend::input::InputEvent {
                                kind: crate::backend::input::EventKind::Keyboard { keycode, state },
                                ..
                            } = event
                            {
                                let esc = u32::from(evdev::KeyCode::KEY_ESC.code());
                                let f1 = u32::from(evdev::KeyCode::KEY_F1.code());
                                let f12 = u32::from(evdev::KeyCode::KEY_F12.code());

                                if *state == input::event::keyboard::KeyState::Pressed {
                                    tracing::trace!("KEY PRESS: {keycode:?}");
                                    match *keycode {
                                        keycode if keycode == esc => self
                                            .event_loop
                                            .emitter()
                                            .emit(crate::token::Token::Exit),

                                        keycode if (f1..=f12).contains(&keycode) => {
                                            let vt = (keycode - f1 + 1) as i32;
                                            tracing::info!(vt, "Requesting VT switch");
                                            if let Err(error) = self.seat.switch_session(vt) {
                                                tracing::error!(
                                                    ?error,
                                                    vt,
                                                    "Failed to switch VT session"
                                                );
                                            }
                                        }

                                        _ => (),
                                    }
                                }

                                if *state == input::event::keyboard::KeyState::Pressed
                                    && *keycode == esc
                                {
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
                    }

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
                        tracing::trace!("EXIT");
                        return Ok(());
                    }
                }
            }
        }
    }
}
