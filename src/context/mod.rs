mod cli;
pub mod logging;
mod path;

#[derive(Debug)]
pub struct Context {
    pub cli: cli::domain::Cli,
    pub path: path::Path,
    pub log_target: logging::LogTarget,
}

impl Context {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        use clap::Parser;
        let cli = cli::domain::Cli::parse();
        cli.process()?;
        let path = path::Path::new(cli.config.clone())?;

        let log_target = logging::LogTarget::new(cli.log_to.as_ref())?;
        Ok(Self {
            cli,
            path,
            log_target,
        })
    }
}

fn to_absolute(path: std::path::PathBuf) -> Result<std::path::PathBuf, crate::error::PathError> {
    let absolute_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|err| crate::error::PathError::Io {
                action: "et current directory",
                path: None,
                source: err,
            })?
            .join(path)
    };

    absolute_path
        .canonicalize()
        .map_err(|err| crate::error::PathError::Io {
            action: "anonicalize path",
            path: Some(absolute_path),
            source: err,
        })
}

fn expand_tilde(path: std::path::PathBuf) -> Result<std::path::PathBuf, crate::error::PathError> {
    let str = path.to_string_lossy();

    if !str.starts_with('~') {
        return Ok(path);
    }

    let home = std::env::var("HOME").map_err(|_| crate::error::PathError::InvalidPath {
        input: path.clone(),
        reason: "HOME environment variable not set",
    })?;

    let expanded = if str == "~" {
        std::path::PathBuf::from(home)
    } else if str.starts_with("~/") {
        std::path::PathBuf::from(home).join(&str[2..])
    } else {
        return Err(crate::error::PathError::InvalidPath {
            input: path,
            reason: "unsupported tilde expansion (Report an issue)",
        });
    };

    Ok(expanded)
}
