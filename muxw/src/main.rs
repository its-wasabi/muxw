#![allow(unused)]

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

// TODO: Think about using references if Token needs to reference bigger blob of data e.g.: if you
// introduce Token::Notify variant it would probably carry notify data which eventually will be
// String you could potentially pass ownership of that string and string itself is mainly size of
// the pointer but its already making it not optimal for events that carry only its variant in the
// Token... Think about it (hell or low performance) (making data indirect two times) Token ->
// NotifyData(String) -> String in heap is also not good
#[derive(Debug, Clone, PartialEq)]
enum Token {
    WaylandSocket,
    WaylandDisplay,
    WaylandClientDisconnected(wayland_server::backend::ClientId),

    DrmUdev,
    DrmCard(backend::drm::DrmCardKey),

    Input(backend::input::InputEvent),
    Config(config::ConfigRequest),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new()?;

    #[cfg(debug_assertions)]
    println!("{context:#?}");

    compositor::Compositor::new(&context)?.run()
}
