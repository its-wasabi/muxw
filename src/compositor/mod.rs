use std::os::fd::AsFd;

mod client;
mod state;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::token::Token>,
    triggered_events: Vec<crate::token::Token>,

    seat: crate::backend::seat::SeatManager,

    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,

    // drm_manager: crate::backend::drm::DrmManager,
    renderer: crate::renderer::Renderer,

    state: state::State,
}

impl Compositor {
    pub fn new(context: &crate::context::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let seat = crate::backend::seat::SeatManager::new(&mut event_loop)?;

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

        // let drm_manager = crate::backend::drm::DrmManager::new(&mut event_loop)?;

        let renderer = crate::renderer::Renderer::new()?;

        let state = state::State::new(context, &event_loop)?;

        event_loop.register_timer(
            crate::event_loop::TimerMode::Delay(std::time::Duration::from_secs(1)),
            crate::token::Token::Shutdown,
        );

        Ok(Self {
            event_loop,
            triggered_events,

            seat,

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
                    crate::token::Token::SeatEvent => {
                        if let Err(error) = self.seat.dispatch() {
                            tracing::error!(?error, "Failed to dispatch libseat seat");
                        }
                    }

                    crate::token::Token::SeatEnable => {
                        tracing::debug!("Seat Enable");
                    }

                    crate::token::Token::SeatDisable => {
                        tracing::debug!("Seat Disable");

                        if let Err(error) = self.seat.disable() {
                            // FIXME: Handle error like ignore seat disable not working at least
                            // drop DRM master lock or something if libseat didn't do already
                            tracing::error!(?error, "Failed to ack Seat Disable");
                        }
                    }

                    crate::token::Token::SeatOpenRequest(open_data) => {
                        self.seat.open_device(open_data)?;
                    }

                    crate::token::Token::SeatCloseRequest(raw_fd) => {
                        self.seat.close_device(raw_fd)?;
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
                    }
                    crate::token::Token::DrmCard(drm_card_key) => {
                        tracing::trace!(?drm_card_key, "DrmCard event triggered");
                    }
                    crate::token::Token::Input(event) => {
                        tracing::debug!(?event, "Input event received");
                    }
                    crate::token::Token::Config(command) => {
                        tracing::debug!(?command, "Config command received");
                    }

                    crate::token::Token::Shutdown => {
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
