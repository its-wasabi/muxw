#[derive(Debug, clap::Parser)]
#[command(name = crate::NAME)]
#[command(author, version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = false)]
#[command(subcommand_value_name = "SUBCOMMAND")]
#[command(subcommand_help_heading = "Subcommands")]
#[command(after_help = "Use \"muxw [SUBCOMMAND] --help\" for more information on a subcommand")]
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
        /// Output in json format
        #[arg(short, long)]
        json: bool,
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

    /// Exit from running muxw instance
    Exit,
}

#[derive(Debug, clap::Subcommand)]
pub enum CliSubQuery {
    /// List available output devices
    Outputs,
    /// List available input devices
    Inputs,
    /// List currently opened windows
    Windows,
    /// Get currently focused window
    FocusedWindow,
    /// Get currently focused workspace
    FocusedWorkspace,
}

pub enum CliHandleOutcome {
    Exit,
    Continue,
}

impl Cli {
    pub fn process(&self) -> Result<CliHandleOutcome, crate::error::CliError> {
        match &self.subcommand {
            // TODO: Rename to "get"
            Some(CliSub::Query { query, json }) => {
                Self::process_query(query, json)?;
                Ok(CliHandleOutcome::Exit)
            }

            // TODO: Rename to "check"
            Some(CliSub::Validate { config }) => {
                Self::process_validate(config)?;
                Ok(CliHandleOutcome::Exit)
            }

            // TODO: Make that compile time code
            Some(CliSub::MakeCompletion { shell, stdout }) => {
                Self::process_make_completion(shell, stdout)?;
                Ok(CliHandleOutcome::Exit)
            }

            _ => Ok(CliHandleOutcome::Continue),
        }
    }

    fn process_query(query: &CliSubQuery, json: &bool) -> Result<(), crate::error::CliError> {
        match query {
            CliSubQuery::Outputs => todo!("Outputs"),
            CliSubQuery::Inputs => todo!("Inputs"),
            CliSubQuery::Windows => todo!("Windows"),
            CliSubQuery::FocusedWindow => todo!("FocusedWindow"),
            CliSubQuery::FocusedWorkspace => todo!("FocusedWorkspace"),
        }
    }

    fn process_validate(config: &Option<std::path::PathBuf>) -> Result<(), crate::error::CliError> {
        todo!("First implement config logic - Validate config:{config:?}");
    }

    // TODO: Move that to system installation process
    fn process_make_completion(
        shell: &Option<clap_complete::Shell>,
        stdout: &bool,
    ) -> Result<(), crate::error::CliError> {
        use clap::CommandFactory;
        let mut cmd = Cli::command();

        let shell = shell.unwrap_or(get_shell()?);

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

fn get_shell() -> Result<clap_complete::Shell, crate::error::CliError> {
    let shell_var = std::env::var("SHELL").map_err(|_| crate::error::CliError::ShellNotDetected)?;
    let shell_name = std::path::Path::new(&shell_var)
        .file_name()
        .and_then(|sh| sh.to_str())
        .ok_or(crate::error::CliError::ShellNotDetected)?;

    match shell_name {
        sh if sh.contains("bash") => Ok(clap_complete::Shell::Bash),
        sh if sh.contains("zsh") => Ok(clap_complete::Shell::Zsh),
        sh if sh.contains("fish") => Ok(clap_complete::Shell::Fish),
        sh if sh.contains("elvish") => Ok(clap_complete::Shell::Elvish),
        _ => Err(crate::error::CliError::ShellNotSupported {
            shell: shell_name.to_string(),
        }),
    }
}

// TODO: make that prefer files that consent require root
fn get_shell_completion_file(
    shell: clap_complete::Shell,
) -> Result<std::fs::File, crate::error::CliError> {
    #[rustfmt::skip]
    let path = std::path::PathBuf::from(match shell {
        clap_complete::Shell::Bash => format!("/usr/share/bash-completion/completions/{}", crate::NAME),
        clap_complete::Shell::Zsh => format!("/usr/share/zsh/site-functions/_{}", crate::NAME),
        clap_complete::Shell::Fish => format!("/usr/share/fish/completions/{}.fish", crate::NAME),
        clap_complete::Shell::Elvish => format!("/usr/share/elvish/lib/{}.elv", crate::NAME),
        clap_complete::Shell::PowerShell => todo!("PowerShell not supported"),
        _ => todo!("Handle that shell: {shell}"),
    });

    if let Some(dir_path) = path.parent() {
        std::fs::create_dir_all(dir_path).map_err(|err| crate::error::CliError::Io {
            action: "create completion directory",
            source: err,
        })?;
    };

    std::fs::File::create(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::PermissionDenied {
            crate::error::CliError::PermissionDenied { path }
        } else {
            crate::error::CliError::Io {
                action: "create completion file",
                source: err,
            }
        }
    })
}
