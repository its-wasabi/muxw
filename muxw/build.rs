#[path = "src/cli/domain.rs"]
mod domain;

use clap::CommandFactory;

fn main() -> Result<(), std::io::Error> {
    run_spell_check();
    build_shell_completion()
}

fn run_spell_check() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=_typos.toml");

    let Ok(output) = std::process::Command::new("typos")
        .args(["--format", "brief", "src"])
        .output()
    else {
        println!(
            "cargo:warning=`typos` not found in PATH — skipping spell check. Install with `cargo install typos-cli`."
        );
        return;
    };

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.trim().is_empty() {
            println!("cargo:warning=typo: {line}");
        }
    }
}

fn build_shell_completion() -> Result<(), std::io::Error> {
    if let Some(outdir) = std::env::var_os("OUT_DIR") {
        let mut cmd = domain::Cli::command();
        let bin_name = std::env!("CARGO_PKG_NAME");

        let shells = [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::Elvish,
            clap_complete::Shell::PowerShell,
        ];

        for shell in shells {
            clap_complete::generate_to(shell, &mut cmd, bin_name, &outdir)?;
        }

        println!(
            "cargo:info=Completion scripts generated in {}",
            outdir.display()
        );
    }

    Ok(())
}
