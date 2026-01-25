use std::io;

#[derive(Debug, clap::Parser)]
#[command(name = crate::NAME)]
#[command(author, version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = false)]
#[command(subcommand_value_name = "SUBCOMMAND")]
#[command(subcommand_help_heading = "Subcommands")]
#[command(after_help = "Use \"ray [SUBCOMMAND] --help\" for more information on a subcommand")]
pub struct Cli {
    /// Set config file or directory path
    #[arg(short, long, value_name = "PATH")]
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

    /// Validate configuration file
    Validate {
        /// Path to config file or directory
        #[arg(short, long, value_name = "PATH")]
        config: Option<std::path::PathBuf>,
    },

    /// Generate and apply completions
    MakeCompletion {
        /// Target shell
        #[arg(long)]
        shell: Option<clap_complete::Shell>,
        /// Print to stdout instead of installing
        #[arg(long)]
        stdout: bool,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum CliSubQuery {
    /// List available output devices
    Outputs,
    /// List available input devices
    Inputs,
    /// List currently opened windows
    Windows,
}

#[derive(Debug)]
pub enum CliError {
    ShellNotSupported,
    ShellNotDetected,
    FileCreation(std::io::Error),
    PermissionDenied,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::ShellNotSupported => write!(f, "Shell not supported"),
            CliError::ShellNotDetected => write!(f, "Could not detect shell"),
            CliError::FileCreation(error) => write!(f, "Failed to create completion file: {error}"),
            CliError::PermissionDenied => {
                write!(f, "Permission denied. You may want to run this with sudo")
            }
        }
    }
}

impl std::error::Error for CliError {}

impl Cli {
    pub fn handle(&self) -> Result<(), CliError> {
        match &self.subcommand {
            Some(CliSub::Query { query }) => Self::handle_query(query)?,
            Some(CliSub::Validate { config }) => Self::handle_validate(config)?,
            Some(CliSub::MakeCompletion { shell, stdout }) => {
                Self::handle_make_completion(shell, stdout)?
            }
            _ => return Ok(()),
        };

        std::process::exit(0)
    }

    fn handle_query(query: &CliSubQuery) -> Result<(), CliError> {
        match query {
            CliSubQuery::Outputs => todo!("Outputs"),
            CliSubQuery::Inputs => todo!("Inputs"),
            CliSubQuery::Windows => todo!("Windows"),
        }
    }

    fn handle_validate(config: &Option<std::path::PathBuf>) -> Result<(), CliError> {
        todo!("First implement config logic - Validate config:{config:?}");
    }

    fn handle_make_completion(
        shell: &Option<clap_complete::Shell>,
        stdout: &bool,
    ) -> Result<(), CliError> {
        use clap::CommandFactory;
        let mut cmd = Cli::command();

        let shell = shell.unwrap_or(get_shell().ok_or(CliError::ShellNotSupported)?);

        if *stdout {
            clap_complete::generate(shell, &mut cmd, crate::NAME, &mut std::io::stdout());
        } else {
            clap_complete::generate(
                shell,
                &mut cmd,
                crate::NAME,
                &mut get_shell_completion_file(shell)?,
            );
        };

        Ok(())
    }
}

fn get_shell() -> Option<clap_complete::Shell> {
    if let Ok(shell_path) = std::env::var("SHELL") {
        let shell_name = std::path::Path::new(&shell_path).file_name()?.to_str()?;
        match shell_name {
            sh if sh.contains("bash") => Some(clap_complete::Shell::Bash),
            sh if sh.contains("zsh") => Some(clap_complete::Shell::Zsh),
            sh if sh.contains("fish") => Some(clap_complete::Shell::Fish),
            sh if sh.contains("elvish") => Some(clap_complete::Shell::Elvish),
            _ => None,
        }
    } else {
        todo!("Implement fallback behaviour");
    }
}

fn get_shell_completion_file(shell: clap_complete::Shell) -> Result<std::fs::File, CliError> {
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
        std::fs::create_dir_all(dir_path).map_err(CliError::FileCreation)?;
    };

    std::fs::File::create(file_path).map_err(|err| {
        if err.kind() == io::ErrorKind::PermissionDenied {
            CliError::PermissionDenied
        } else {
            CliError::FileCreation(err)
        }
    })
}
