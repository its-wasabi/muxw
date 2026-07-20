const VERSION: (u32, u32, u32) = helpers::parse_version(env!("CARGO_PKG_VERSION"));

mod backend;
mod compositor;
mod config;
mod context;
mod error;
mod event_loop;
mod helpers;
mod renderer;
mod token;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = context::Context::new()?;
    let _log_guard = context::logging::init_logging(&context.log_target)?;

    #[cfg(debug_assertions)]
    println!("{context:#?}");

    compositor::Compositor::new(&context)?.run()
}
