use std::os::fd::AsFd;

mod client;
mod state;

pub struct Compositor {
    event_loop: crate::event_loop::EventLoop<crate::token::Token>,
    triggered_events: Vec<crate::token::Token>,

    seat: libseat::Seat,
    seat_devices: std::collections::HashMap<std::os::unix::io::RawFd, libseat::Device>,

    display: wayland_server::Display<state::State>,
    socket: wayland_server::ListeningSocket,

    // drm_manager: crate::backend::drm::DrmManager,
    renderer: crate::renderer::Renderer,

    state: state::State,
}

impl Compositor {
    pub fn new(context: &crate::Context) -> Result<Self, Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;
        let triggered_events = Vec::new();

        println!("BRUH");
        let seat_sender = event_loop.channel_sender();
        let mut seat = libseat::Seat::open(move |_, seat_event| match seat_event {
            libseat::SeatEvent::Enable => seat_sender.send(crate::token::Token::SeatEnable),
            libseat::SeatEvent::Disable => seat_sender.send(crate::token::Token::SeatDisable),
        })?;
        event_loop.register_source(
            &seat.get_fd()?,
            polling::PollMode::Level,
            crate::token::Token::SeatEvent,
        )?;
        println!("OK");

        // TODO: Fine tune the capacity
        let seat_devices = std::collections::HashMap::with_capacity(10);

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

        Ok(Self {
            event_loop,
            triggered_events,

            seat,
            seat_devices,

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
                match token {
                    crate::token::Token::SeatEvent => {
                        if let Err(error) = self.seat.dispatch(0) {
                            unimplemented!("IMPLEMENT REal Logging");
                        }
                    }

                    crate::token::Token::SeatEnable => {
                        // NOTE: You already have DRM master lock via libseat
                        // NOTE: you should call something like resume on drm_manager
                        println!("Seat Enable");
                    }

                    crate::token::Token::SeatDisable => {
                        // NOTE: you should call something like pause on drm_manager
                        println!("Seat Disable");

                        if let Err(error) = self.seat.disable() {
                            // FIXME: Handle error like ignore seat disable not working at least
                            // drop DRM master lock or something if libseat didn't do already
                            unimplemented!("Implement logging");
                        }
                    }

                    crate::token::Token::SeatOpenRequest(open_data) => {
                        match self.seat.open_device(&&open_data.path) {
                            Ok(device) => {
                                // device only implements AsFd. We MUST duplicate it for libinput
                                // so libinput gets its own descriptor to manage and drop.
                                match rustix::io::dup(device.as_fd()) {
                                    Ok(owned_fd) => {
                                        use std::os::unix::io::AsRawFd;
                                        let raw_fd = owned_fd.as_raw_fd();

                                        // Store the `Device` lease in the map keyed by the new RawFd.
                                        // If we don't store it, it drops, and the seat daemon might revoke it!
                                        self.seat_devices.insert(raw_fd, device);

                                        let _ = open_data.reply.send(Ok(owned_fd));
                                    }
                                    Err(_) => {
                                        // Dup failed. Immediately return the lease to libseat.
                                        if let Err(e) = self.seat.close_device(device) {
                                            log::error!(
                                                "Failed to close device after dup error: {:?}",
                                                e
                                            );
                                        }
                                        let _ = open_data.reply.send(Err(-1));
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Seat failed to open device {:?}: {:?}",
                                    open_data.path,
                                    e
                                );
                                let _ = open_data.reply.send(Err(-13));
                            }
                        }
                    }

                    crate::token::Token::SeatCloseRequest(raw_fd) => {
                        if let Some(device) = self.seat_devices.remove(&raw_fd) {
                            if let Err(error) = self.seat.close_device(device) {
                                unimplemented!("REAL LOGGING");
                            }
                        }
                    }

                    crate::token::Token::WaylandSocket => {
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
                    crate::token::Token::WaylandDisplay => {
                        println!("EV::(WaylandDisplay)");
                        self.display.dispatch_clients(&mut self.state)?;
                        self.display.flush_clients()?;
                    }
                    crate::token::Token::WaylandClientDisconnected(id) => {
                        println!("EV::(WaylandClientDisconnected): {id:?}");
                    }
                    crate::token::Token::DrmUdev => {
                        println!("EV::(DrmUdevMonitor)");
                        // self.drm_manager.dispatch_udev(&mut self.event_loop);
                    }
                    crate::token::Token::DrmCard(drm_card_key) => {
                        // self.drm_manager.dispatch_card(drm_card_key);
                    }
                    crate::token::Token::Input(event) => {
                        println!("EV::(Input): {event:?}");
                    }
                    crate::token::Token::Config(command) => {
                        println!("EV::(Config):{command:#?}");
                    }
                }
            }
        }
    }
}
