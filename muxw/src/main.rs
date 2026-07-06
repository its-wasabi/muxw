const DEFAULT_CONFIG: &str = /* lua */
    r#"
print("INSIDE LUA")
Mux.bind("W", Mux.motion.focus.up);
Mux.bind("S", Mux.motion.focus.down);
Mux.bind("D", Mux.motion.focus.right);
Mux.bind("A", Mux.motion.focus.left);
print("LUA DONE")
"#;

const VERSION: (u32, u32, u32) = helpers::parse_version(env!("CARGO_PKG_VERSION"));

mod backend;
mod cli;
mod compositor;
mod config;
mod error;
mod event_loop;
mod helpers;
mod path;
mod renderer;

#[derive(Debug)]
struct Context {
    cli: cli::domain::Cli,
    path: path::Path,
}

impl Context {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use clap::Parser;
        let cli = cli::domain::Cli::parse();
        cli.process()?;
        let path = path::Path::new(cli.config.clone())?;
        Ok(Self { cli, path })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    WaylandSocket,
    WaylandDisplay,
    WaylandClientDisconnected(wayland_server::backend::ClientId),
    Input(backend::input::InputEvent),
    Config(config::ConfigRequest),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new()?;

    #[cfg(debug_assertions)]
    println!("{context:#?}");

    compositor::Compositor::new(&context)?.run()
}
