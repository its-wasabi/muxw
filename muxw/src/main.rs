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

mod cli;
mod compositor;
mod config;
mod error;
mod event_loop;
mod helpers;
mod path;

#[derive(Debug)]
struct Context {
    cli: cli::domain::Cli,
    path: path::Path,
}

impl Context {
    fn new() -> Result<Self, crate::error::PathError> {
        use clap::Parser;
        let cli = cli::domain::Cli::parse();
        cli.process().expect("Failed to process");
        let path = path::Path::new(cli.config.clone())?;
        Ok(Self { cli, path })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Config(config::ConfigRequest),
    Input(compositor::input::InputEvent),

    WaylandSocket,
    WaylandDisplay,
    WaylandClientDisconnected(wayland_server::backend::ClientId),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new()?;

    #[cfg(debug_assertions)]
    println!("{:#?}", context);

    compositor::Compositor::new(&context)?.run()
}
