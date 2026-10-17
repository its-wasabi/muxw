mod backend;
mod compositor;
mod config;
mod context;
mod event_loop;
mod meta;
mod renderer;
mod token;
mod wayland;

const APP_NAME: &str = meta::get_app_name();
const APP_NAME_CSTR: &std::ffi::CStr = meta::get_app_name_cstr();
const APP_VERSION: meta::Version = meta::get_app_version();

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
    let context = context::Context::new()?;
    let _log_guard = context::logging::init_logging(&context.log_target)?;

    #[cfg(debug_assertions)]
    println!("{context:#?}");

    let compositor = compositor::Compositor::new(&context).inspect_err(|error| {
        tracing::error!(error);
    })?;

    compositor.run().inspect_err(|error| {
        tracing::error!(error);
    })?;

    Ok(())
}
