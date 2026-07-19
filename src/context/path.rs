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
            let config_dir = super::to_absolute(path)?;
            let config_file = config_dir.join(Self::get_config_filename());
            (config_dir, config_file)
        } else {
            let config_file = super::to_absolute(path)?;
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
                let path = super::expand_tilde(path)?;
                if path.is_dir() {
                    let config_dir = super::to_absolute(path)?;
                    let config_file = config_dir.join(Self::get_config_filename());
                    return Ok(Some((config_dir, config_file)));
                } else {
                    let config_file = super::to_absolute(path)?;
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
                let config_dir =
                    std::path::PathBuf::from(xdg_config).join(env!("CARGO_CRATE_NAME"));
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
                    .join(env!("CARGO_CRATE_NAME"));
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

        let current_dir_file = current_dir.join(Self::get_config_filename());

        Ok((current_dir, current_dir_file))
    }
}
