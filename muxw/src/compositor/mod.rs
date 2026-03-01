use wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use wayland_server::protocol::{
    wl_compositor::WlCompositor, wl_output::WlOutput, wl_seat::WlSeat, wl_shm::WlShm,
};

mod globals;

pub struct Muxw {
    display: wayland_server::Display<Self>,
    globals: globals::Globals,

    config: crate::config::ConfigContext,
}

impl Muxw {
    pub fn new() -> Result<Self, crate::error::InitError> {
        let display =
            wayland_server::Display::new().map_err(|err| crate::error::InitError::Wayland {
                action: "init wayland display",
                source: err,
            })?;
        let globals = globals::Globals::new(&display.handle());

        let config = crate::config::ConfigContext::new(&crate::PATH.get().config_file)?;

        Ok(Self {
            display,
            globals,
            config,
        })
    }

    pub fn run(&mut self) {}
}

struct Ex {
    opt1: bool,
}

impl wayland_server::GlobalDispatch<WlCompositor, ()> for Muxw {
    fn bind(
        state: &mut Self,
        handle: &wayland_server::DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<WlCompositor>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}

impl wayland_server::GlobalDispatch<WlShm, ()> for Muxw {
    fn bind(
        state: &mut Self,
        handle: &wayland_server::DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<WlShm>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}
impl wayland_server::GlobalDispatch<XdgWmBase, ()> for Muxw {
    fn bind(
        state: &mut Self,
        handle: &wayland_server::DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<XdgWmBase>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}
impl wayland_server::GlobalDispatch<WlSeat, ()> for Muxw {
    fn bind(
        state: &mut Self,
        handle: &wayland_server::DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<WlSeat>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}
impl wayland_server::GlobalDispatch<WlOutput, ()> for Muxw {
    fn bind(
        state: &mut Self,
        handle: &wayland_server::DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<WlOutput>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}

pub struct Socket {}

// NOTE: Read that
// pub fn new_auto() -> Result<ListeningSocketSource, BindError> {
//     // Try socket numbers 1-32. Remember the upper bound of Range is exclusive.
//     //
//     // We don't try wayland-0 due since clients may connect to the wrong compositor. Clients these days
//     // should be connecting based off the WAYLAND_DISPLAY or WAYLAND_SOCKET environment variables.
//     let socket = ListeningSocket::bind_auto("wayland", 1..33)?;
//
//     info!(name = ?socket.socket_name(), "Created new socket");
//
//     Ok(ListeningSocketSource {
//         socket: Generic::new(socket, Interest::READ, Mode::Level),
//     })
// }
//
// /// Creates a new listening socket with the specified name.
// pub fn with_name(name: &str) -> Result<ListeningSocketSource, BindError> {
//     let socket = ListeningSocket::bind(name)?;
//     info!(name = ?socket.socket_name(), "Created new socket");
//
//     Ok(ListeningSocketSource {
//         socket: Generic::new(socket, Interest::READ, Mode::Level),
//     })
// }
//
// /// Returns the name of the listening socket.
// pub fn socket_name(&self) -> &OsStr {
//     self.socket.get_ref().socket_name().unwrap()
// }

// NOTE: Read that implementation
// /// Attempt to bind a listening socket from a sequence of names
// ///
// /// This method will repeatedly try to bind sockets in the form `{basename}-{n}` for values of `n`
// /// yielded from the provided range and returns the first one that succeeds.
// ///
// /// This method will acquire an associate lockfile. The socket will be created in the
// /// directory pointed to by the `XDG_RUNTIME_DIR` environment variable.
// pub fn bind_auto(
//     basename: &str,
//     range: impl IntoIterator<Item = usize>,
// ) -> Result<Self, BindError> {
//     for i in range {
//         // early return on any error except AlreadyInUse
//         match Self::bind(format!("{basename}-{i}")) {
//             Ok(socket) => return Ok(socket),
//             Err(BindError::RuntimeDirNotSet) => return Err(BindError::RuntimeDirNotSet),
//             Err(BindError::PermissionDenied) => return Err(BindError::PermissionDenied),
//             Err(BindError::Io(e)) => return Err(BindError::Io(e)),
//             Err(BindError::AlreadyInUse) => {}
//         }
//     }
//     Err(BindError::AlreadyInUse)
// }
