#[derive(Debug, clap::Parser)]
// TODO:  #[command(name = crate::NAME)]
#[command(author, version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = false)]
#[command(subcommand_value_name = "SUBCOMMAND")]
#[command(subcommand_help_heading = "Subcommands")]
#[command(after_help = "Use \"muxw [SUBCOMMAND] --help\" for more information on a subcommand")]
pub struct Cli {
    /// Set config file or directory path
    #[arg(short, long, value_name = "PATH", global = true)]
    pub config: Option<std::path::PathBuf>,

    /// Set config file or directory path
    #[arg(short, long, value_name = "WAYLAND SOCKET", global = true)]
    pub socket: Option<String>,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, clap::Subcommand)]
pub enum CliCommand {
    /// Query the compositor for runtime info
    Query {
        #[command(subcommand)]
        target: QueryTarget,

        /// output in JSON format
        #[arg(short, long)]
        json: bool,
    },

    /// check configuration file
    Check,
}

#[derive(Debug, clap::Subcommand)]
pub enum QueryTarget {
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
