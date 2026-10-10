pub mod domain;

impl domain::Cli {
    pub fn process(&self) -> Result<(), super::error::Error> {
        match &self.command {
            Some(domain::CliCommand::Query { target }) => {
                Self::query(target, self.json)?;
                std::process::exit(0);
            }
            None => Ok(()),
        }
    }

    fn query(target: &domain::QueryTarget, _json: bool) -> Result<(), super::error::Error> {
        let name = match target {
            domain::QueryTarget::Outputs => "outputs",
            domain::QueryTarget::Inputs => "inputs",
            domain::QueryTarget::Windows => "windows",
            domain::QueryTarget::FocusedWindow => "focused-window",
            domain::QueryTarget::FocusedWorkspace => "focused-workspace",
        };
        Err(super::error::Error::QueryNotImplemented { target: name })
    }
}
