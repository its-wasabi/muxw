pub mod args;

impl args::Cli {
    pub fn process(&self) {
        match &self.command {
            Some(args::CliCommand::Query { target }) => {
                Self::query(target, self.json);
                std::process::exit(0);
            }
            None => (),
        }
    }

    fn query(target: &args::QueryTarget, _json: bool) -> &'static str {
        match target {
            args::QueryTarget::Outputs => "outputs",
            args::QueryTarget::Inputs => "inputs",
            args::QueryTarget::Windows => "windows",
            args::QueryTarget::FocusedWindow => "focused-window",
            args::QueryTarget::FocusedWorkspace => "focused-workspace",
        }
    }
}
