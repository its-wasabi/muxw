pub mod domain;

pub enum CliHandleOutcome {
    Exit,
    Continue,
}

impl domain::Cli {
    pub fn process(&self) -> Result<CliHandleOutcome, crate::error::CliError> {
        match &self.command {
            Some(domain::CliCommand::Query { target, json }) => {
                Self::process_query(target, json)?;
                Ok(CliHandleOutcome::Exit)
            }

            Some(domain::CliCommand::Check) => {
                Self::process_check(&self.config)?;
                Ok(CliHandleOutcome::Exit)
            }

            None => Ok(CliHandleOutcome::Continue),
        }
    }

    fn process_query(
        query: &domain::QueryTarget,
        json: &bool,
    ) -> Result<(), crate::error::CliError> {
        match query {
            domain::QueryTarget::Outputs => todo!("Outputs"),
            domain::QueryTarget::Inputs => todo!("Inputs"),
            domain::QueryTarget::Windows => todo!("Windows"),
            domain::QueryTarget::FocusedWindow => todo!("FocusedWindow"),
            domain::QueryTarget::FocusedWorkspace => todo!("FocusedWorkspace"),
        }
    }

    fn process_check(config: &Option<std::path::PathBuf>) -> Result<(), crate::error::CliError> {
        todo!("First implement config logic - Validate config:{config:?}");
    }
}
