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

    /// Exit from ray
    Exit,
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
            Some(CliSub::Exit) => std::process::exit(0),
            None => (),
        }
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
        todo!("Implement make completion logic - Completion sehll:{shell:?} stdout:{stdout}");
    }
}
