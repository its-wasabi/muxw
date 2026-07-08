use std::os::fd::AsFd;

mod client;
mod seat;
mod state;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::Token>,
    triggered_events: Vec<crate::Token>,

    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,

    drm_manager: crate::backend::drm::DrmManager,
    renderer: crate::renderer::Renderer,

    state: state::State,
}

impl Compositor {
    pub fn new(context: &crate::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

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
            crate::Token::WaylandSocket,
        )?;
        event_loop.register_source(
            &display.backend().poll_fd().as_fd(),
            polling::PollMode::Edge,
            crate::Token::WaylandDisplay,
        )?;

        let drm_manager = crate::backend::drm::DrmManager::new(&mut event_loop)?;

        let renderer = crate::renderer::Renderer::new()?;

        let state = state::State::new(context, &event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,

            display,
            socket,

            drm_manager,
            renderer,

            state,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in self.triggered_events.drain(..) {
                match token {
                    crate::Token::WaylandSocket => {
                        if let Some(stream) = self.socket.accept()? {
                            let mut display_handle = self.display.handle();
                            let client_state = client::ClientState {
                                event_sender: self.event_loop.channel_sender(),
                            };

                            let client = display_handle
                                .insert_client(stream, std::sync::Arc::new(client_state))?;

                            if let Ok(credentials) = client.get_credentials(&display_handle) {
                                println!(
                                    "New client: PID: {}, UID: {}, GID: {}",
                                    credentials.pid, credentials.uid, credentials.gid
                                );
                            }
                        }
                    }
                    crate::Token::WaylandDisplay => {
                        println!("EV::(WaylandDisplay)");
                        self.display.dispatch_clients(&mut self.state)?;
                        self.display.flush_clients()?;
                    }
                    crate::Token::WaylandClientDisconnected(id) => {
                        println!("EV::(WaylandClientDisconnected): {id:?}");
                    }
                    crate::Token::DrmUdev => {
                        println!("EV::(DrmUdevMonitor)");
                        self.drm_manager.dispatch_udev(&mut self.event_loop);
                    }
                    crate::Token::DrmCard(drm_card_key) => {
                        self.drm_manager.dispatch_card(drm_card_key);
                    }
                    crate::Token::Input(event) => {
                        println!("EV::(Input): {event:?}");
                    }
                    crate::Token::Config(command) => {
                        println!("EV::(Config):{command:#?}");
                    }
                }
            }
        }
    }
}
