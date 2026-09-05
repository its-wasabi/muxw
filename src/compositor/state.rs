pub struct State {}

impl State {
    pub fn new(
        context: &crate::context::Context,
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }
}

impl wayland_server::GlobalDispatch<wayland_server::protocol::wl_compositor::WlCompositor, ()>
    for State
{
    fn bind(
        _state: &mut Self,
        _handle: &wayland_server::DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<wayland_server::protocol::wl_compositor::WlCompositor>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        println!("BIND");
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<wayland_server::protocol::wl_compositor::WlCompositor, ()> for State {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &wayland_server::protocol::wl_compositor::WlCompositor,
        request: <wayland_server::protocol::wl_compositor::WlCompositor as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        println!("Received wl_compositor request: {request:?}");
    }
}
