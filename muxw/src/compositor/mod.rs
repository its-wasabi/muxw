use std::os::fd::AsFd;

mod client_state;
mod compositor_state;
pub mod input;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::Token>,
    triggered_events: Vec<crate::Token>,

    display: wayland_server::Display<compositor_state::CompositorState>,
    socket: wayland_server::ListeningSocket,

    state: compositor_state::CompositorState,
}

impl Compositor {
    pub fn new(context: &crate::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        let mut display = wayland_server::Display::new()?;
        let display_handle = display.handle();
        display_handle
            .create_global::<compositor_state::CompositorState, wayland_server::protocol::wl_compositor::WlCompositor, ()>(
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

        let state = compositor_state::CompositorState::new(context, &mut event_loop)?;

        Ok(Self {
            event_loop,
            triggered_events,

            display,
            socket,

            state,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.event_loop.dispatch(&mut self.triggered_events)?;

            for token in self.triggered_events.drain(..) {
                match token {
                    crate::Token::Input(event) => self.state.input_manager.process_event(&event),
                    crate::Token::Config(command) => println!("EV::CONFIG::{command:#?}"),

                    crate::Token::WaylandSocket => {
                        if let Some(stream) = self.socket.accept()? {
                            let client_state = client_state::ClientState {
                                event_sender: self.event_loop.channel_sender(),
                            };

                            self.display
                                .handle()
                                .insert_client(stream, std::sync::Arc::new(client_state))?;
                        }
                    }

                    crate::Token::WaylandDisplay => {
                        println!("Wayland (DISPLAY)");
                        self.display.dispatch_clients(&mut self.state)?;
                        self.display.flush_clients()?;
                    }
                    crate::Token::WaylandClientDisconnected(id) => {
                        println!("WC-DISCONNECTED: {id:?}");
                    }
                }
            }
        }
    }
}
