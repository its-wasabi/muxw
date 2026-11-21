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

pub const DEFAULT_CONFIG: &str = /* lua */
    r#"
print("INSIDE LUA")
Mux.bind("W", Mux.motion.focus.up);
Mux.bind("S", Mux.motion.focus.down);
Mux.bind("D", Mux.motion.focus.right);
Mux.bind("A", Mux.motion.focus.left);
print("LUA DONE")
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = startup::Context::new()?;
    let _log_guard = startup::logging::init_logging(&context.log_target)?;

    compositor::Compositor::new(&context)
        .inspect_err(|error| {
            tracing::error!(error);
        })?
        .run()
        .inspect_err(|error| {
            tracing::error!(error);
        })?;

    Ok(())
}
