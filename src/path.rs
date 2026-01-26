fn to_absolute(path: std::path::PathBuf) -> Result<std::path::PathBuf, PathError> {
    let absolute_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|err| PathError::Io {
                action: "et current directory",
                path: None,
                source: err,
            })?
            .join(path)
    };

    absolute_path.canonicalize().map_err(|err| PathError::Io {
        action: "anonicalize path",
        path: Some(absolute_path),
        source: err,
    })
}

fn expand_tilde(path: std::path::PathBuf) -> Result<std::path::PathBuf, PathError> {
    let str = path.to_string_lossy();

    if !str.starts_with("~") {
        return Ok(path);
    }

    let home = std::env::var("HOME").map_err(|_| PathError::InvalidPath {
        input: path.clone(),
        reason: "HOME environment variable not set",
    })?;

    let expanded = if str == "~" {
        std::path::PathBuf::from(home)
    } else if str.starts_with("~/") {
        std::path::PathBuf::from(home).join(&str[2..])
    } else {
        return Err(PathError::InvalidPath {
            input: path,
            reason: "unsupported tilde expansion (Report an issue)",
        });
    };

    Ok(expanded)
}

#[derive(Debug)]
pub struct Path {
    config_dir: std::path::PathBuf,
    config_file: std::path::PathBuf,
}

#[derive(Debug)]
pub enum PathError {
    /// User supplied a path that is syntactically invalid
    InvalidPath {
        input: std::path::PathBuf,
        reason: &'static str,
    },
    /// A config path was expected but not found
    ConfigNotFound {
        origin: &'static str,
        path: std::path::PathBuf,
    },
    /// Any IO failure with context
    Io {
        action: &'static str,
        path: Option<std::path::PathBuf>,
        source: std::io::Error,
    },
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::InvalidPath { input, reason } => {
                write!(f, "Invalid path \"{}\": {reason}", input.display())
            }
            PathError::ConfigNotFound { origin, path } => {
                write!(
                    f,
                    "Config not found  (from {origin}) at \"{}\"",
                    path.display()
                )
            }
            PathError::Io {
                action,
                path,
                source,
            } => {
                if let Some(path) = path {
                    write!(f, "Failed to {action} \"{}\": {source}", path.display())
                } else {
                    write!(f, "Failed to {action}: {source}")
                }
            }
        }
    }
}

impl std::error::Error for PathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PathError::Io {
                action,
                path,
                source,
            } => Some(source),
            _ => None,
        }
    }
}

impl Path {
    pub fn new(cli: &crate::cli::Cli) -> Result<Self, PathError> {
        let input_config: Option<(std::path::PathBuf, std::path::PathBuf)> = match &cli.config {
            Some(cli_path) => Some(Self::get_config_paths_from_cli(cli_path.clone())?),
            None => Self::get_config_paths_from_env()?,
        };

        let (config_dir, config_file) = if let Some(input) = input_config {
            input
        } else {
            Self::get_config_paths_from_defaults()?
        };

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    fn get_config_filename() -> String {
        String::from("config.lua")
    }

    fn get_config_paths_from_cli(
        path: std::path::PathBuf,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), PathError> {
        let config = if path.is_dir() {
            let config_dir = to_absolute(path)?;
            let config_file = config_dir.join(Self::get_config_filename());
            (config_dir, config_file)
        } else {
            let config_file = to_absolute(path)?;
            let config_dir = config_file
                .parent()
                .ok_or(PathError::InvalidPath {
                    input: config_file.clone(),
                    reason: "path has no parent directory",
                })?
                .to_path_buf();
            (config_dir, config_file)
        };

        Ok(config)
    }

    fn get_config_paths_from_env()
    -> Result<Option<(std::path::PathBuf, std::path::PathBuf)>, PathError> {
        #[allow(clippy::collapsible_if)]
        if let Ok(ray_env) = std::env::var("RAY_CONFIG_PATH") {
            if !ray_env.is_empty() {
                let path = std::path::PathBuf::from(ray_env);
                let path = expand_tilde(path)?;
                if path.is_dir() {
                    let config_dir = to_absolute(path)?;
                    let config_file = config_dir.join(Self::get_config_filename());
                    return Ok(Some((config_dir, config_file)));
                } else {
                    let config_file = to_absolute(path)?;
                    let config_dir = config_file
                        .parent()
                        .ok_or(PathError::InvalidPath {
                            input: config_file.clone(),
                            reason: "path has no parent directory",
                        })?
                        .to_path_buf();

                    return Ok(Some((config_dir, config_file)));
                }
            }
        }

        Ok(None)
    }

    fn get_config_paths_from_defaults()
    -> Result<(std::path::PathBuf, std::path::PathBuf), PathError> {
        if let Some(config) = Self::get_config_paths_from_xdg_var() {
            return Ok(config);
        } else if let Some(config) = Self::get_config_paths_from_home_var() {
            return Ok(config);
        }

        Self::get_config_paths_from_current_dir()
    }

    fn get_config_paths_from_xdg_var() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        #[allow(clippy::collapsible_if)]
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg_config.is_empty() {
                let config_dir = std::path::PathBuf::from(xdg_config).join(crate::NAME);
                let config_file = config_dir.join(Self::get_config_filename());
                return Some((config_dir, config_file));
            }
        };
        None
    }

    fn get_config_paths_from_home_var() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        #[allow(clippy::collapsible_if)]
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                let config_dir = std::path::PathBuf::from(home)
                    .join(".config")
                    .join(crate::NAME);
                let config_file = config_dir.join(Self::get_config_filename());
                return Some((config_dir, config_file));
            }
        }
        None
    }

    fn get_config_paths_from_current_dir()
    -> Result<(std::path::PathBuf, std::path::PathBuf), PathError> {
        let current_dir = std::env::current_dir().map_err(|err| PathError::Io {
            action: "Failed to get current directory",
            path: None,
            source: err,
        })?;
        here!(
            "No config dir found creating .{} config dir at {}",
            crate::NAME,
            current_dir.display()
        );

        let current_dir_file = current_dir.join(Self::get_config_filename());

        Ok((current_dir, current_dir_file))
    }
}
