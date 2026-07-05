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

#[derive(Debug)]
pub struct Path {
    pub config_dir: std::path::PathBuf,
    pub config_file: std::path::PathBuf,
}

impl Path {
    pub fn new(
        config_override: Option<std::path::PathBuf>,
    ) -> Result<Self, crate::error::PathError> {
        let input_config = match config_override {
            Some(config_path) => Some(Self::get_config_paths_from_cli(config_path)?),
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
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), crate::error::PathError> {
        let config = if path.is_dir() {
            let config_dir = to_absolute(path)?;
            let config_file = config_dir.join(Self::get_config_filename());
            (config_dir, config_file)
        } else {
            let config_file = to_absolute(path)?;
            let config_dir = config_file
                .parent()
                .ok_or(crate::error::PathError::InvalidPath {
                    input: config_file.clone(),
                    reason: "path has no parent directory",
                })?
                .to_path_buf();
            (config_dir, config_file)
        };

        Ok(config)
    }

    fn get_config_paths_from_env()
    -> Result<Option<(std::path::PathBuf, std::path::PathBuf)>, crate::error::PathError> {
        #[allow(clippy::collapsible_if)]
        if let Ok(muxw_env) = std::env::var("MUXW_CONFIG_PATH") {
            if !muxw_env.is_empty() {
                let path = std::path::PathBuf::from(muxw_env);
                let path = expand_tilde(path)?;
                if path.is_dir() {
                    let config_dir = to_absolute(path)?;
                    let config_file = config_dir.join(Self::get_config_filename());
                    return Ok(Some((config_dir, config_file)));
                } else {
                    let config_file = to_absolute(path)?;
                    let config_dir = config_file
                        .parent()
                        .ok_or(crate::error::PathError::InvalidPath {
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
    -> Result<(std::path::PathBuf, std::path::PathBuf), crate::error::PathError> {
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
    -> Result<(std::path::PathBuf, std::path::PathBuf), crate::error::PathError> {
        let current_dir = std::env::current_dir().map_err(|err| crate::error::PathError::Io {
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
