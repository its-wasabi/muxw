use std::os::fd::AsFd;

mod client;
mod state;

pub struct WaylandServer {
    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,
    state: state::State,
}

impl WaylandServer {
    pub fn new(
        context: &crate::context::Context,
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
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
        event_loop.register_source(&socket.as_fd(), polling::PollMode::Edge, Event::Socket)?;
        event_loop.register_source(
            &display.backend().poll_fd().as_fd(),
            polling::PollMode::Edge,
            Event::Display,
        )?;

        let state = state::State::new(event_loop)?;

        Ok(Self {
            display,
            socket,
            state,
        })
    }

    pub fn dispatch(
        &mut self,
        event: Event,
        event_loop: &crate::event_loop::EventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            Event::Socket => self.handle_socket(event_loop),
            Event::Display => self.handle_display(),
            Event::ClientDisconnected(client_id) => {
                tracing::info!(?client_id, "Wayland client disconnected");
                Ok(())
            }
        }
    }

    fn handle_socket(
        &self,
        event_loop: &crate::event_loop::EventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(stream) = self.socket.accept()? {
            let mut display_handle = self.display.handle();
            let client_state = client::Client {
                event_sender: event_loop.sender(),
            };

            let client = display_handle.insert_client(stream, std::sync::Arc::new(client_state))?;

            if let Ok(credentials) = client.get_credentials(&display_handle) {
                tracing::info!(
                    pid = credentials.pid,
                    uid = credentials.uid,
                    gid = credentials.gid,
                    "New Wayland client connected"
                );
            } else {
                tracing::info!("New Wayland client connected (credentials unavailable)");
            }
        }

        Ok(())
    }

    fn handle_display(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::trace!("Dispatching Wayland display clients");
        self.display.dispatch_clients(&mut self.state)?;
        self.display.flush_clients()?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Socket,
    Display,
    ClientDisconnected(wayland_server::backend::ClientId),
}
