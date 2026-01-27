#[derive(Debug)]
pub struct Globals {
    pub compositor: wayland_server::protocol::wl_compositor::WlCompositor,
    pub shm: wayland_server::protocol::wl_shm::WlShm,
    pub seat: wayland_server::protocol::wl_seat::WlSeat,
    pub outputs: wayland_server::protocol::wl_output::WlOutput,
    pub xdg_wm_base: wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase,
}

impl Globals {
    pub fn new() -> Self {
        todo!()
    }
}
