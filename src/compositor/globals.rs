use wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use wayland_server::protocol::wl_compositor::WlCompositor;
use wayland_server::protocol::wl_output::WlOutput;
use wayland_server::protocol::wl_seat::WlSeat;
use wayland_server::protocol::wl_shm::WlShm;
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New};

#[derive(Debug)]
pub struct Globals;

impl Globals {
    pub fn new(display_handle: &DisplayHandle) -> Self {
        // create_global signature is:
        // pub fn create_global<D, I, F>(version: u32, filter: F)
        // where D: GlobalDispatch<I, GlobalData> + 'static
        //       I: Resource + 'static
        //       F: Fn(&Client) -> bool + 'static

        // The third generic is a FILTER function
        // The filter determines which clients can see this global

        display_handle.create_global::<crate::compositor::Compositor, WlCompositor, _>(6, ());

        display_handle.create_global::<crate::compositor::Compositor, WlShm, _>(1, ());

        display_handle.create_global::<crate::compositor::Compositor, XdgWmBase, _>(5, ());

        display_handle.create_global::<crate::compositor::Compositor, WlSeat, _>(9, ());

        display_handle.create_global::<crate::compositor::Compositor, WlOutput, _>(4, ());

        Self
    }
}
