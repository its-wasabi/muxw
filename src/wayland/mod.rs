use std::os::fd::AsFd;

mod client;
pub mod error;
mod state;

pub struct Wayland {
    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,
    state: state::State,
}

impl Wayland {
    pub fn new(
        context: &crate::context::Context,
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<Self, error::Error> {
        let mut display = wayland_server::Display::new().map_err(error::Error::CreateDisplay)?;
        display.handle().create_global::<
            state::State,
            wayland_server::protocol::wl_compositor::WlCompositor,
            (),
        >(5, ());

        let socket = match &context.cli.socket {
            Some(name) => wayland_server::ListeningSocket::bind(name).map_err(|source| {
                error::Error::BindSocket {
                    name: name.clone(),
                    source: Box::new(source),
                }
            })?,
            None => wayland_server::ListeningSocket::bind_auto("wayland", 0..=100)
                .map_err(|source| error::Error::BindAutoSocket(Box::new(source)))?,
        };

        event_loop
            .register_io(
                &socket.as_fd(),
                crate::event_loop::Interest::Readable,
                polling::PollMode::Edge,
                Event::Socket,
            )
            .map_err(error::Error::RegisterSocket)?;
        event_loop
            .register_io(
                &display.backend().poll_fd().as_fd(),
                crate::event_loop::Interest::Readable,
                polling::PollMode::Edge,
                Event::Display,
            )
            .map_err(error::Error::RegisterDisplay)?;

        Ok(Self {
            display,
            socket,
            state: state::State::new(event_loop),
        })
    }

    pub fn dispatch(
        &mut self,
        event: Event,
        event_loop: &crate::event_loop::EventLoop,
    ) -> Result<(), error::Error> {
        match event {
            Event::Socket => self.accept_clients(event_loop),
            Event::Display => self.dispatch_clients(),
            Event::ClientDisconnected(client_id) => {
                tracing::info!(?client_id, "Wayland client disconnected");
                Ok(())
            }
        }
    }

    fn accept_clients(
        &self,
        event_loop: &crate::event_loop::EventLoop,
    ) -> Result<(), error::Error> {
        // The socket is registered edge-triggered, so every pending connection
        // has to be drained now: no further wakeup comes for ones left queued.
        while let Some(stream) = self.socket.accept().map_err(error::Error::AcceptClient)? {
            let mut display_handle = self.display.handle();
            let client_data = client::Client {
                event_sender: event_loop.sender(),
            };

            let client = display_handle
                .insert_client(stream, std::sync::Arc::new(client_data))
                .map_err(error::Error::InsertClient)?;

            match client.get_credentials(&display_handle) {
                Ok(credentials) => tracing::info!(
                    pid = credentials.pid,
                    uid = credentials.uid,
                    gid = credentials.gid,
                    "New Wayland client connected"
                ),
                Err(_) => {
                    tracing::info!("New Wayland client connected (credentials unavailable)");
                }
            }
        }

        Ok(())
    }

    fn dispatch_clients(&mut self) -> Result<(), error::Error> {
        tracing::trace!("Dispatching Wayland display clients");
        self.display
            .dispatch_clients(&mut self.state)
            .map_err(error::Error::DispatchClients)?;
        self.display
            .flush_clients()
            .map_err(error::Error::FlushClients)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Socket,
    Display,
    ClientDisconnected(wayland_server::backend::ClientId),
}
