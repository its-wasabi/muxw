#[derive(Debug)]
pub enum CliError {
    ShellNotSupported {
        shell: String,
    },
    ShellNotDetected,
    PermissionDenied {
        path: std::path::PathBuf,
    },
    Io {
        action: &'static str,
        source: std::io::Error,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShellNotSupported { shell } => write!(f, "{shell} is not supported"),
            Self::ShellNotDetected => write!(f, "Could not detect the current shell"),
            Self::PermissionDenied { path } => write!(
                f,
                "Permission denied while writing \"{}\". Try running as root.",
                path.display()
            ),
            Self::Io { action, source } => write!(f, "Failed to {action}: {source}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { action, source } => Some(source),
            _ => None,
        }
    }
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
            Self::InvalidPath { input, reason } => {
                write!(f, "Invalid path \"{}\": {reason}", input.display())
            }
            Self::ConfigNotFound { origin, path } => {
                write!(
                    f,
                    "Config not found  (from {origin}) at \"{}\"",
                    path.display()
                )
            }
            Self::Io {
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
            Self::Io {
                action,
                path,
                source,
            } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum InitError {
    Wayland {
        action: &'static str,
        source: wayland_server::backend::InitError,
    },
    Mlua {
        action: &'static str,
    },

    Io {
        action: &'static str,
        path: Option<std::path::PathBuf>,
        source: std::io::Error,
    },
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wayland { action, source } => {
                write!(f, "Failed to {action}: {source}")
            }
            Self::Mlua { action } => write!(f, "Failed to {action}"),
            Self::Io {
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

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Wayland { action, source } => Some(source),
            Self::Mlua { action } => None,
            Self::Io {
                action,
                path,
                source,
            } => Some(source),
        }
    }
}
