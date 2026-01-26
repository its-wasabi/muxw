fn to_absolute(path: std::path::PathBuf) -> Result<std::path::PathBuf, std::io::Error> {
    if path.is_absolute() {
        return path.canonicalize();
    }
    std::env::current_dir()?.join(path).canonicalize()
}

fn expand_tilde(path: std::path::PathBuf) -> Result<std::path::PathBuf, std::io::Error> {
    todo!();
}

#[derive(Debug)]
pub struct Path {
    config_dir: std::path::PathBuf,
    config_file: std::path::PathBuf,
}

#[derive(Debug)]
pub enum PathError {
    ConfigNotFound(String, String),

    InvalidPath(String),

    Todo,
    IoError(std::io::Error),
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::ConfigNotFound(from, at) => {
                write!(f, "Config path from {from} not found at {at}")
            }
            PathError::InvalidPath(path) => write!(f, "Invalid config path: {path}"),
            PathError::IoError(err) => write!(f, "IO: {err}"),
            PathError::Todo => panic!("TODO: Make that proper error type"),
        }
    }
}

impl std::error::Error for PathError {}

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
            let config_dir = to_absolute(path).map_err(PathError::IoError)?;
            let config_file = config_dir.join(Self::get_config_filename());
            (config_dir, config_file)
        } else {
            let config_file = to_absolute(path).map_err(PathError::IoError)?;
            let config_dir = config_file.parent().ok_or(PathError::Todo)?.to_path_buf();
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
                // FIXME: Normalize path here (to avoid errors on ~)
                if path.is_dir() {
                    let config_dir = to_absolute(path).map_err(PathError::IoError)?;
                    let config_file = config_dir.join(Self::get_config_filename());
                    return Ok(Some((config_dir, config_file)));
                } else {
                    let config_file = to_absolute(path).map_err(PathError::IoError)?;
                    let config_dir = config_file.parent().ok_or(PathError::Todo)?.to_path_buf();
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
        let current_dir = std::env::current_dir().map_err(PathError::IoError)?;
        here!(
            "No config dir found creating .{} config dir at {}",
            crate::NAME,
            current_dir.display()
        );

        let current_dir_file = current_dir.join(Self::get_config_filename());

        Ok((current_dir, current_dir_file))
    }
}
