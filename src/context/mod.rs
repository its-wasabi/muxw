mod cli;
pub mod error;
pub mod logging;
mod path;

#[derive(Debug)]
pub struct Context {
    pub cli: cli::domain::Cli,
    pub path: path::Path,
    pub log_target: logging::LogTarget,
}

impl Context {
    pub fn new() -> Result<Self, error::Error> {
        use clap::Parser;
        let cli = cli::domain::Cli::parse();
        cli.process()?;
        let path = path::Path::new(cli.config.clone())?;
        let log_target = logging::LogTarget::new(cli.log_to.as_deref())?;
        Ok(Self {
            cli,
            path,
            log_target,
        })
    }
}

fn non_empty_env(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

fn to_absolute(path: std::path::PathBuf) -> Result<std::path::PathBuf, error::Error> {
    std::path::absolute(&path).map_err(|source| error::Error::MakeAbsolute {
        input: path,
        source,
    })
}

fn expand_tilde(path: std::path::PathBuf) -> Result<std::path::PathBuf, error::Error> {
    let Some(text) = path.to_str() else {
        return Ok(path);
    };
    if !text.starts_with('~') {
        return Ok(path);
    }

    let home = non_empty_env("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| error::Error::HomeNotSet {
            input: path.clone(),
        })?;

    if text == "~" {
        Ok(home)
    } else if let Some(rest) = text.strip_prefix("~/") {
        Ok(home.join(rest))
    } else {
        Err(error::Error::UnsupportedTilde { input: path })
    }
}
