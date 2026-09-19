mod client;
mod state;

use std::os::fd::AsFd;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop,
    triggered_events: Vec<crate::token::Token>,

    seat_manager: crate::backend::seat::SeatManager,
    input_manager: crate::backend::input::InputManager,
    // drm_manager: crate::backend::drm::DrmManager,
    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,

    state: state::State,

    renderer: crate::renderer::Renderer,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let seat_manager = crate::backend::seat::SeatManager::new(&mut event_loop)?;
        let input_manager =
            crate::backend::input::InputManager::new(&mut event_loop, seat_manager.handle())?;

        let mut display = wayland_server::Display::new()?;
        let display_handle = display.handle();
        display_handle
            .create_global::<state::State, wayland_server::protocol::wl_compositor::WlCompositor, ()>(
                5,
                (),
            );
        let socket = if let Some(socket_name) = &context.cli.socket {
            wayland_server::ListeningSocket::bind(socket_name)?
        } else {
            wayland_server::ListeningSocket::bind_auto("wayland", 0..=100)?
        };
        event_loop.register_source(
            &socket.as_fd(),
            polling::PollMode::Edge,
            crate::token::Token::WaylandSocket,
        )?;
        event_loop.register_source(
            &display.backend().poll_fd().as_fd(),
            polling::PollMode::Edge,
            crate::token::Token::WaylandDisplay,
        )?;

        let renderer = crate::renderer::Renderer::new()?;

        // let drm_manager = crate::backend::drm::DrmManager::new(&mut event_loop, &renderer)?;

        let state = state::State::new(context, &mut event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,

            seat_manager,
            input_manager,

            display,
            socket,

            // drm_manager,
            renderer,

            state,
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

                    crate::token::Token::Libinput => {
                        self.input_manager.dispatch()?;
                    }

                    crate::token::Token::WaylandSocket => {
                        if let Some(stream) = self.socket.accept()? {
                            let mut display_handle = self.display.handle();
                            let client_state = client::ClientState {
                                event_sender: self.event_loop.sender(),
                            };

                            let client = display_handle
                                .insert_client(stream, std::sync::Arc::new(client_state))?;

                            if let Ok(credentials) = client.get_credentials(&display_handle) {
                                tracing::info!(
                                    pid = credentials.pid,
                                    uid = credentials.uid,
                                    gid = credentials.gid,
                                    "New Wayland client connected"
                                );
                            } else {
                                tracing::info!(
                                    "New Wayland client connected (credentials unavailable)"
                                );
                            }
                        }
                    }
                    crate::token::Token::WaylandDisplay => {
                        tracing::trace!("Dispatching Wayland display clients");
                        self.display.dispatch_clients(&mut self.state)?;
                        self.display.flush_clients()?;
                    }
                    crate::token::Token::WaylandClientDisconnected(id) => {
                        tracing::info!(?id, "Wayland client disconnected");
                    }
                    crate::token::Token::DrmUdev => {
                        tracing::trace!("DrmUdev monitor event triggered");
                        // self.drm_manager
                        //     .dispatch_udev(&mut self.event_loop, &self.renderer);
                    }
                    // crate::token::Token::DrmCard(drm_card_key) => {
                    //     tracing::trace!(?drm_card_key, "DrmCard event triggered");
                    //     // self.drm_manager.dispatch_card(
                    //     //     drm_card_key,
                    //     //     &self.renderer,
                    //     //     [0.0, 0.0, 0.0, 1.0],
                    //     // );
                    // }
                    crate::token::Token::Input(event) => {
                        let esc = evdev::KeyCode::KEY_ESC.code();
                        let _space = evdev::KeyCode::KEY_SPACE.code();
                        tracing::debug!(?event, "Input event received");
                        if let crate::backend::input::InputEvent {
                            kind:
                                crate::backend::input::InputEventKind::Keyboard {
                                    keycode,
                                    state: input::event::keyboard::KeyState::Pressed,
                                },
                            ..
                        } = event
                        {
                            if keycode == esc as u32 {
                                self.event_loop.emiter().emit(crate::token::Token::Shutdown);
                            }
                        }

                        // self.drm_manager.paint_all(&self.renderer, [r, g, b, 1.0]);
                    }
                    crate::token::Token::Config(command) => {
                        tracing::debug!(?command, "Config command received");
                    }

                    crate::token::Token::Shutdown => {
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
