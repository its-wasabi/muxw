#[derive(Debug)]
pub enum Error {
    ReadConfig {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    WriteDefaultConfig {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    Thread(std::io::Error),
    ThreadGone,
    Lua(mlua::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadConfig { path, .. } => {
                write!(f, "failed to read config file \"{}\"", path.display())
            }
            Self::WriteDefaultConfig { path, .. } => {
                write!(f, "failed to write default config \"{}\"", path.display())
            }
            Self::Thread(_) => f.write_str("failed to start config thread"),
            Self::ThreadGone => f.write_str("config thread is no longer running"),
            Self::Lua(_) => f.write_str("failed to load config"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ReadConfig { error: source, .. } => source.source(),
            Error::WriteDefaultConfig { error: source, .. } => source.source(),
            Error::Thread(error) => error.source(),
            Error::ThreadGone => None,
            Error::Lua(error) => error.source(),
        }
    }
}

impl From<mlua::Error> for Error {
    fn from(error: mlua::Error) -> Self {
        Self::Lua(error)
    }
}
