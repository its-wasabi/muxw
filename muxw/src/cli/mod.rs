pub mod domain;

impl domain::Cli {
    pub fn process(&self) -> Result<(), crate::error::CliError> {
        match &self.command {
            Some(domain::CliCommand::Query { target }) => {
                Self::query(target, self.json)?;
                std::process::exit(0);
            }

            None => Ok(()),
        }
    }

    fn query(query: &domain::QueryTarget, json: bool) -> Result<(), crate::error::CliError> {
        match query {
            domain::QueryTarget::Outputs => todo!("Outputs"),
            domain::QueryTarget::Inputs => todo!("Inputs"),
            domain::QueryTarget::Windows => todo!("Windows"),
            domain::QueryTarget::FocusedWindow => todo!("FocusedWindow"),
            domain::QueryTarget::FocusedWorkspace => todo!("FocusedWorkspace"),
        }
    }
}
