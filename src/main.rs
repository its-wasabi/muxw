mod backend;
mod build_info;
mod compositor;
mod config;
mod event;
mod event_loop;
mod renderer;
mod startup;
mod wayland;

const APP_NAME: &str = build_info::get_app_name();
const APP_NAME_CSTR: &std::ffi::CStr = build_info::get_app_name_cstr();
const APP_VERSION: build_info::Version = build_info::get_app_version();

const CONFIG_FILENAME: &str = "init.lua";
pub const DEFAULT_CONFIG: &str = /* lua */
    r#"
Mux.bind("Super + W", Mux.motion.focus.up);
Mux.bind("Super + S", Mux.motion.focus.down);
Mux.bind("Super + D", Mux.motion.focus.right);
Mux.bind("Super + A", Mux.motion.focus.left);

Mux.bind("Super + O > T", function()
    Mux.open("kitty")
end);

Mux.bind("Super + Backspace", Mux.exit);
"#;

fn main() -> eyre::Result<()> {
    let context = startup::Context::new()?;
    compositor::Compositor::new(&context)
        .inspect_err(|error| tracing::error!(?error))?
        .run()
        .inspect_err(|error| tracing::error!(?error))?;

    Ok(())
}
