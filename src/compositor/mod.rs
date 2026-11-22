mod seat;

pub struct Compositor {
    wayland: crate::wayland::Wayland,
    drm: crate::backend::drm::Drm,
    renderer: crate::renderer::Renderer,
    config: crate::config::Handle,
    input: crate::backend::input::Libinput,

    event_loop: crate::event_loop::EventLoop<crate::Event>,
    triggered_events: Vec<crate::Event>,

    // NOTE: Must be dropped after other components since it handles device close calls
    seat: crate::backend::session::Seat,
}

impl Compositor {
    pub fn new(context: &crate::startup::Context) -> eyre::Result<Self> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let config = crate::config::Service::spawn(context, &mut event_loop)?;
        let seat = crate::backend::session::Seat::new(&mut event_loop)?;
        let renderer = crate::renderer::Renderer::new()?;
        let drm = crate::backend::drm::Drm::new(seat.handle(), &mut event_loop)?;
        let input = crate::backend::input::Libinput::new(&mut event_loop, seat.handle())?;

        let wayland = crate::wayland::Wayland::new(context, &mut event_loop)?;

        Ok(Self {
            wayland,
            drm,
            renderer,
            config,
            input,

            event_loop,
            triggered_events,

            seat,
        })
    }

    pub fn run(mut self) -> eyre::Result<()> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;
            for event in self.triggered_events.drain(..) {
                let _span_guard = tracing::debug_span!("event", ?event).entered();

                match event {
                    crate::Event::Session(event) => match event {
                        crate::backend::session::Event::Dispatch => {
                            if let Err(error) = self.seat.dispatch() {
                                tracing::error!(?error, "Failed to dispatch libseat seat");
                            }
                        }

                        crate::backend::session::Event::Enable => {
                            if let Err(error) = self.input.resume() {
                                tracing::error!(?error, "Failed to resume libinput");
                            }

                            if let Err(error) = self.drm.resume() {
                                tracing::error!(?error, "Failed to resume DRM");
                            }
                        }

                        crate::backend::session::Event::Disable => {
                            self.input.suspend();
                            self.drm.suspend()?;

                            if let Err(error) = self.seat.disable() {
                                tracing::error!(?error, "Failed to ack Seat Disable");
                            }
                        }
                    },

                    crate::Event::Input(crate::backend::input::Event::Dispatch) => {
                        self.input.dispatch(|event| {
                            if let crate::backend::input::InputEvent {
                                kind:
                                    crate::backend::input::EventKind::KeyboardKey {
                                        keycode,
                                        pressed,
                                        ..
                                    },
                                ..
                            } = event
                            {
                                let esc = u32::from(evdev::KeyCode::KEY_ESC.code());
                                let f1 = u32::from(evdev::KeyCode::KEY_F1.code());
                                let f12 = u32::from(evdev::KeyCode::KEY_F12.code());

                                if pressed {
                                    match keycode {
                                        keycode if keycode == esc => {
                                            self.event_loop.emitter().emit(crate::Event::Exit)
                                        }

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
                            }
                        })?;
                    }

                    crate::Event::Drm(event) => {
                        self.drm.dispatch(event, &mut self.event_loop)?;
                    }

                    crate::Event::Wayland(event) => {
                        self.wayland.dispatch(event, &self.event_loop)?;
                    }

                    crate::Event::Config(command) => {
                        eprintln!("Apply config logic here");
                    }

                    crate::Event::Exit => {
                        tracing::trace!("EXIT");
                        return Ok(());
                    }
                }
            }
        }
    }
}
