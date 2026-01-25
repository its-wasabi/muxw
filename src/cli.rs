#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Set config directory/file path
    #[arg(short, long, value_name = "PATH", global = true)]
    pub config: Option<std::path::PathBuf>,
    #[command(subcommand)]
    pub subcommand: Option<CliSub>,
}

#[derive(Debug, clap::Subcommand)]
pub enum CliSub {
    /// Query runtime info
    Query {
        #[command(subcommand)]
        query: CliSubQuery,
    },

    /// Validate config correctness
    Validate,

    // Generate and apply completion for selected shell
    MakeCompletion {
        #[arg(long)]
        shell: Option<clap_complete::Shell>,
        // Output completion to stdout instead of apply
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum CliSubQuery {
    /// List output devices
    Outputs,
    /// List input devices
    Input,
    /// List opened windows
    Windows,
}

impl Cli {
    pub fn handle(&self) {
        match &self.subcommand {
            Some(CliSub::Query { query }) => Self::handle_query(query),
            Some(CliSub::Validate) => Self::handle_validate(&self.config),
            Some(CliSub::MakeCompletion { shell, stdout }) => {
                Self::handle_make_completion(shell, stdout)
            }
            None => return,
        }
        std::process::exit(0);
    }

    fn handle_query(query: &CliSubQuery) {
        match query {
            CliSubQuery::Outputs => todo!("Outputs"),
            CliSubQuery::Input => todo!("Inputs"),
            CliSubQuery::Windows => todo!("Windows"),
        }
    }

    fn handle_validate(config: &Option<std::path::PathBuf>) {
        todo!("First implement config logic - Validate config:{config:?}");
    }

    fn handle_make_completion(shell: &Option<clap_complete::Shell>, stdout: &bool) {
        use clap::CommandFactory;
        let mut cmd = Cli::command();

        let shell = shell.unwrap_or(get_shell().unwrap_or_else(|| todo!("Shell not supported")));

        if *stdout {
            clap_complete::generate(shell, &mut cmd, crate::NAME, &mut std::io::stdout());
        } else {
            clap_complete::generate(
                shell,
                &mut cmd,
                crate::NAME,
                &mut get_shell_completion_file(shell),
            );
        }
    }
}

fn get_shell() -> Option<clap_complete::Shell> {
    if let Ok(shell_path) = std::env::var("SHELL") {
        let shell_name = std::path::Path::new(&shell_path).file_name()?.to_str()?;
        match shell_name {
            shell if shell.contains("bash") => Some(clap_complete::Shell::Bash),
            shell if shell.contains("zsh") => Some(clap_complete::Shell::Zsh),
            shell if shell.contains("fish") => Some(clap_complete::Shell::Fish),
            shell if shell.contains("elvish") => Some(clap_complete::Shell::Elvish),
            _ => None,
        }
    } else {
        todo!("Implement fallback behaviour");
    }
}

fn get_shell_completion_file(shell: clap_complete::Shell) -> std::fs::File {
    #[rustfmt::skip]
    let file_path = std::path::PathBuf::from(match shell {
        clap_complete::Shell::Bash => format!("/usr/share/bash-completion/completions/{}", crate::NAME),
        clap_complete::Shell::Zsh => format!("/usr/share/zsh/site-functions/_{}", crate::NAME),
        clap_complete::Shell::Fish => format!("/usr/share/fish/completions/{}.fish", crate::NAME),
        clap_complete::Shell::Elvish => format!("/usr/share/elvish/lib/{}.elv", crate::NAME),
        clap_complete::Shell::PowerShell => todo!("PowerShell not supported"),
        _ => todo!("Handle that shell: {shell}"),
    });

    if let Some(dir_path) = file_path.parent() {
        std::fs::create_dir_all(dir_path);
    };

    match std::fs::File::create(file_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("You may want to run that with sudo");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Failed to create completion file");
            std::process::exit(1);
        }
    }
}
