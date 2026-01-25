#[derive(Debug)]
pub struct Path {
    config_dir: std::path::PathBuf,
    config_file: std::path::PathBuf,
}

#[derive(Debug)]
pub enum PathError {
    ConfigNotFound,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathError::ConfigNotFound => write!(f, "Failed to resolve config path"),
        }
    }
}

impl std::error::Error for PathError {}

impl Path {
    pub fn new(cli: &crate::cli::Cli) -> Result<Self, PathError> {
        let config_dir = match &cli.config {
            Some(path) => path.clone(),
            None => Self::get_config_dir().ok_or(PathError::ConfigNotFound)?,
        };
        let config_file = config_dir.join(Self::get_config_filename());

        Ok(Self {
            config_dir,
            config_file,
        })
    }

    pub fn get_config_dir() -> Option<std::path::PathBuf> {
        #[allow(clippy::collapsible_if)]
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg_config.is_empty() {
                return Some(std::path::PathBuf::from(xdg_config).join(crate::NAME));
            }
        };

        #[allow(clippy::collapsible_if)]
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(
                    std::path::PathBuf::from(home)
                        .join(".config")
                        .join(crate::NAME),
                );
            }
        }

        std::env::current_dir().ok().map(|path| {
            here!("No config dir found using .{} dir here", crate::NAME);
            path.join(format!(".{}", crate::NAME))
        })
    }

    pub fn get_config_filename() -> String {
        format!("{}.lua", crate::NAME)
    }
}
