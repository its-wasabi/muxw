pub mod cli;
pub mod logging;
mod path;

#[derive(Debug)]
pub struct Context {
    pub cli: cli::args::Cli,
    pub path: path::Paths,
}

impl Context {
    pub fn new() -> eyre::Result<Self> {
        use clap::Parser;

        logging::init();

        let cli = cli::args::Cli::parse();
        let path = path::Paths::new(cli.config.clone())?;
        Ok(Self { cli, path })
    }
}
