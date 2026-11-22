use eyre::WrapErr;
use std::os::fd::AsFd;

mod client;
mod protocol;

pub struct Wayland {
    socket_handle: evl::IoHandle<crate::Event>,
    display_handle: evl::IoHandle<crate::Event>,

    display: wayland_server::Display<protocol::ProtocolState>,
    socket: wayland_server::ListeningSocket,
    state: protocol::ProtocolState,
}

impl Wayland {
    pub fn new(
        context: &crate::startup::Context,
        event_loop: &mut evl::EventLoop<crate::Event>,
    ) -> eyre::Result<Self> {
        let mut display =
            wayland_server::Display::new().wrap_err("failed to create Wayland display")?;
        display.handle().create_global::<
            protocol::ProtocolState,
            wayland_server::protocol::wl_compositor::WlCompositor,
            (),
        >(5, ());

        let socket = match &context.cli.socket {
            Some(name) => wayland_server::ListeningSocket::bind(name)
                .wrap_err_with(|| format!("failed to bind Wayland socket '{name}'"))?,
            None => wayland_server::ListeningSocket::bind_auto("wayland", 0..=100)
                .wrap_err("failed to auto-bind Wayland socket")?,
        };

        let socket_handle = event_loop
            .register_fd(&socket.as_fd(), evl::Interest::Readable, Event::Socket)
            .wrap_err("failed to register Wayland socket in event loop")?;

        let display_handle = event_loop
            .register_fd(
                &display.backend().poll_fd().as_fd(),
                evl::Interest::Readable,
                Event::Display,
            )
            .wrap_err("failed to register Wayland display in event loop")?;

        Ok(Self {
            display,
            socket,
            state: protocol::ProtocolState::new(event_loop),

            socket_handle,
            display_handle,
        })
    }

    pub fn dispatch(
        &mut self,
        event: Event,
        event_loop: &evl::EventLoop<crate::Event>,
    ) -> eyre::Result<()> {
        match event {
            Event::Socket => self.accept_clients(event_loop),
            Event::Display => self.dispatch_clients(),
            Event::ClientDisconnected(client_id) => {
                tracing::info!(?client_id, "Wayland client disconnected");
                Ok(())
            }
        }
    }

    fn accept_clients(&self, event_loop: &evl::EventLoop<crate::Event>) -> eyre::Result<()> {
        // The socket is registered edge-triggered, so every pending connection
        // has to be drained now: no further wakeup comes for ones left queued.
        while let Some(stream) = self
            .socket
            .accept()
            .wrap_err("failed to accept Wayland client")?
        {
            let mut display_handle = self.display.handle();
            let client_data = client::Client {
                event_sender: event_loop.sender(),
            };

            let client = display_handle
                .insert_client(stream, std::sync::Arc::new(client_data))
                .wrap_err("failed to insert Wayland client")?;

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

    fn dispatch_clients(&mut self) -> eyre::Result<()> {
        tracing::trace!("Dispatching Wayland display clients");
        self.display
            .dispatch_clients(&mut self.state)
            .wrap_err("failed to dispatch Wayland clients")?;
        self.display
            .flush_clients()
            .wrap_err("failed to flush Wayland clients")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Socket,
    Display,
    ClientDisconnected(wayland_server::backend::ClientId),
}

impl Into<crate::Event> for Event {
    fn into(self) -> crate::Event {
        crate::Event::Wayland(self)
    }
}
