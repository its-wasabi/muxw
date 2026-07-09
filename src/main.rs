// TODO: Think about exposing wasm runtime of the compositor and allowing for writing extensions

#![allow(unused)]

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
mod token;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new()?;

    #[cfg(debug_assertions)]
    println!("{context:#?}");

    compositor::Compositor::new(&context)?.run()
}
