use clap::CommandFactory;
use clap_complete::{Shell, generate_to};
use std::env;
use std::io::Error;

#[path = "src/cli/domain.rs"]
mod domain;

fn main() -> Result<(), Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let mut cmd = domain::Cli::command();
    let bin_name = env!("CARGO_PKG_NAME");

    let shells = [
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::Elvish,
        Shell::PowerShell,
    ];

    for shell in shells {
        generate_to(shell, &mut cmd, bin_name, &outdir)?;
    }

    // TODO: find nicer way to display that (not as warning)
    println!("cargo:warning=Completion scripts generated in {:?}", outdir);

    Ok(())
}
