#[derive(Debug, clap::Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = false)]
#[command(subcommand_value_name = "SUBCOMMAND")]
#[command(subcommand_help_heading = "Subcommands")]
#[command(after_help = "Use \"muxw [SUBCOMMAND] --help\" for more information on a subcommand")]
pub struct Cli {
    /// Set config file or directory path
    #[arg(short, long, value_name = "PATH", global = true)]
    pub config: Option<std::path::PathBuf>,

    /// Set the Wayland socket name
    // TODO: Think about removing that option, socket should be set in config (if even set)
    #[arg(short, long, value_name = "NAME", global = true)]
    pub socket: Option<String>,

    /// Output in JSON format
    #[arg(short, long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, clap::Subcommand)]
pub enum CliCommand {
    /// Query compositor for runtime info
    Query {
        #[command(subcommand)]
        target: QueryTarget,
    },
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
