pub struct Client {
    pub event_sender: evl::Sender<crate::Event>,
}

impl wayland_server::backend::ClientData for Client {
    fn initialized(&self, _client_id: wayland_server::backend::ClientId) {
        println!("Initialized");
    }

    fn disconnected(
        &self,
        client_id: wayland_server::backend::ClientId,
        _reason: wayland_server::backend::DisconnectReason,
    ) {
        if let Err(error) = self
            .event_sender
            .send(super::Event::ClientDisconnected(client_id))
        {
            tracing::error!(?error);
        }
    }
}
