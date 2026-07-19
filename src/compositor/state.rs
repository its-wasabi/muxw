pub struct State {
    pub config_command_sender: crossbeam_channel::Sender<crate::config::ConfigCommand>,
    pub input_manager: crate::backend::input::InputManager,
}

impl State {
    pub(super) fn new(
        context: &crate::context::Context,
        event_loop: &crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config_command_sender =
            crate::config::Config::spawn(&context.path.config_file, event_loop.sender())?;

        let input_manager = crate::backend::input::InputManager::new(event_loop.sender())?;

        Ok(Self {
            config_command_sender,
            input_manager,
        })
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
