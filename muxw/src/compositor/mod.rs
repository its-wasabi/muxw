#![allow(clippy::unwrap_used)]

use wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use wayland_server::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_seat::WlSeat, wl_shm::WlShm,
};

pub struct Muxw {
    display: wayland_server::Display<Self>,
    config: crate::config::SharedConfig,
}

impl Muxw {
    pub fn new() -> Result<Self, crate::error::InitError> {
        let (config_handle, config) =
            crate::config::ConfigContext::spawn(&crate::PATH.config_file)?;

        #[cfg(debug_assertions)]
        {
            here!("First config state: {:#?}", config.load());

            // for _ in 0..5 {
            //     config_handle.reset().unwrap().wait();
            //     here!("After reset: {:#?}", config.load());
            // }

            // for _ in 0..2 {
            //     config_handle.reload(None).unwrap().wait();
            //     here!("After reload: {:#?}", config.load());
            // }

            config_handle
                .event(crate::config::api::event::Event::new(
                    crate::config::api::event::EventKind::Keyboard(
                        crate::config::api::event::keyboard::KeyboardEvent::Added {
                            name: "no".into(),
                            seat: "second".into(),
                            port: "3".into(),
                        },
                    ),
                ))
                .unwrap()
                .wait();
        }

        // let globals = globals::Globals::new(&display.handle());
        // let mut ev = muxw_event_loop::event_loop::EventLoop::new().unwrap();
        // ev.register(&crate::config::CONFIG.keyboard, |field| {
        //     println!("\n\n\nHELLO\n\n\n");
        //     true
        // });

        let display =
            wayland_server::Display::new().map_err(|err| crate::error::InitError::Wayland {
                action: "init wayland display",
                source: err,
            })?;

        // let epoll_fd = libc::epoll_create1(libc::EPOLL_CLOEXEC);
        // let input = crate::input::InputManager::new(epoll_fd).unwrap();
        // let events = [muxw_epoll::Event::blank(); 32];
        // loop {
        //     let n = muxw_epoll::wait(epoll_fd, None, &mut events);
        //     for event in &events[..n] {
        //         let fd = event.fd();
        //         if fd == input.fd() {
        //             input.dispatch(|key| {
        //                 compositor_state.handle_key(key);
        //             });
        //         }
        //     }
        // }

        Ok(Self { display, config })
    }
}

// pub struct Socket {}
//
// // NOTE: Read that
// // pub fn new_auto() -> Result<ListeningSocketSource, BindError> {
// //     // Try socket numbers 1-32. Remember the upper bound of Range is exclusive.
// //     //
// //     // We don't try wayland-0 due since clients may connect to the wrong compositor. Clients these days
// //     // should be connecting based off the WAYLAND_DISPLAY or WAYLAND_SOCKET environment variables.
// //     let socket = ListeningSocket::bind_auto("wayland", 1..33)?;
// //
// //     info!(name = ?socket.socket_name(), "Created new socket");
// //
// //     Ok(ListeningSocketSource {
// //         socket: Generic::new(socket, Interest::READ, Mode::Level),
// //     })
// // }
// //
// // /// Creates a new listening socket with the specified name.
// // pub fn with_name(name: &str) -> Result<ListeningSocketSource, BindError> {
// //     let socket = ListeningSocket::bind(name)?;
// //     info!(name = ?socket.socket_name(), "Created new socket");
// //
// //     Ok(ListeningSocketSource {
// //         socket: Generic::new(socket, Interest::READ, Mode::Level),
// //     })
// // }
// //
// // /// Returns the name of the listening socket.
// // pub fn socket_name(&self) -> &OsStr {
// //     self.socket.get_ref().socket_name().unwrap()
// // }
//
// // NOTE: Read that implementation
// // /// Attempt to bind a listening socket from a sequence of names
// // ///
// // /// This method will repeatedly try to bind sockets in the form `{basename}-{n}` for values of `n`
// // /// yielded from the provided range and returns the first one that succeeds.
// // ///
// // /// This method will acquire an associate lockfile. The socket will be created in the
// // /// directory pointed to by the `XDG_RUNTIME_DIR` environment variable.
// // pub fn bind_auto(
// //     basename: &str,
// //     range: impl IntoIterator<Item = usize>,
// // ) -> Result<Self, BindError> {
// //     for i in range {
// //         // early return on any error except AlreadyInUse
// //         match Self::bind(format!("{basename}-{i}")) {
// //             Ok(socket) => return Ok(socket),
// //             Err(BindError::RuntimeDirNotSet) => return Err(BindError::RuntimeDirNotSet),
// //             Err(BindError::PermissionDenied) => return Err(BindError::PermissionDenied),
// //             Err(BindError::Io(e)) => return Err(BindError::Io(e)),
// //             Err(BindError::AlreadyInUse) => {}
// //         }
// //     }
// //     Err(BindError::AlreadyInUse)
// // }
