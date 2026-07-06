pub struct ClientState {
    pub event_sender: crate::event_loop::EventSender<crate::Token>,
}

impl wayland_server::backend::ClientData for ClientState {
    fn initialized(&self, _client_id: wayland_server::backend::ClientId) {
        println!("Initialized");
    }

    fn disconnected(
        &self,
        client_id: wayland_server::backend::ClientId,
        _reason: wayland_server::backend::DisconnectReason,
    ) {
        self.event_sender
            .send(crate::Token::WaylandClientDisconnected(client_id));
    }
}
