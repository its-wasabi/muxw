use std::os::unix::ffi::OsStrExt;

const CONFIG_FILENAME: &str = "config.lua";
const APP_DIR: &str = env!("CARGO_CRATE_NAME");

#[derive(Debug)]
pub struct Path {
    pub config_dir: std::path::PathBuf,
    pub config_file: std::path::PathBuf,
}

impl Path {
    pub fn new(config_override: Option<std::path::PathBuf>) -> Result<Self, super::error::Error> {
        match config_override {
            Some(path) => Self::from_user_path(path),
            None => match Self::from_env()? {
                Some(paths) => Ok(paths),
                None => Self::from_defaults(),
            },
        }
    }

    /// A user-supplied path is either the config directory or the config file itself.
    /// Existing directories and paths ending in "/" count as directories, so a
    /// not-yet-created config directory is not mistaken for a file.
    fn from_user_path(path: std::path::PathBuf) -> Result<Self, super::error::Error> {
        let path = super::to_absolute(path)?;

        if path.is_dir() || path.as_os_str().as_bytes().ends_with(b"/") {
            let config_file = path.join(CONFIG_FILENAME);
            return Ok(Self {
                config_dir: path,
                config_file,
            });
        }

        let config_dir = path
            .parent()
            .ok_or_else(|| super::error::Error::NoParentDirectory {
                input: path.clone(),
            })?
            .to_path_buf();

        Ok(Self {
            config_dir,
            config_file: path,
        })
    }

    fn from_env() -> Result<Option<Self>, super::error::Error> {
        let Some(value) = super::non_empty_env("MUXW_CONFIG_PATH") else {
            return Ok(None);
        };

        let path = super::expand_tilde(std::path::PathBuf::from(value))?;
        Self::from_user_path(path).map(Some)
    }

    fn from_defaults() -> Result<Self, super::error::Error> {
        let config_dir = if let Some(xdg) = super::non_empty_env("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg).join(APP_DIR)
        } else if let Some(home) = super::non_empty_env("HOME") {
            std::path::PathBuf::from(home).join(".config").join(APP_DIR)
        } else {
            std::env::current_dir().map_err(super::error::Error::CurrentDir)?
        };

        let config_file = config_dir.join(CONFIG_FILENAME);
        Ok(Self {
            config_dir,
            config_file,
        })
    }
}
