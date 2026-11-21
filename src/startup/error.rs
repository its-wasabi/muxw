#[derive(Debug)]
pub enum Error {
    CurrentDir(std::io::Error),
    MakeAbsolute {
        input: std::path::PathBuf,
        source: std::io::Error,
    },
    HomeNotSet {
        input: std::path::PathBuf,
    },
    UnsupportedTilde {
        input: std::path::PathBuf,
    },
    NoParentDirectory {
        input: std::path::PathBuf,
    },
    QueryNotImplemented {
        target: &'static str,
    },
    CreateLogDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    OpenLogFile {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    InitLogger(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentDir(_) => f.write_str("failed to get current directory"),
            Self::MakeAbsolute { input, .. } => {
                write!(f, "failed to make \"{}\" absolute", input.display())
            }
            Self::HomeNotSet { input } => {
                write!(f, "cannot expand \"{}\": HOME is not set", input.display())
            }
            Self::UnsupportedTilde { input } => write!(
                f,
                "unsupported tilde expansion in \"{}\" (only \"~\" and \"~/...\" are supported)",
                input.display()
            ),
            Self::NoParentDirectory { input } => write!(
                f,
                "config path \"{}\" has no parent directory",
                input.display()
            ),
            Self::QueryNotImplemented { target } => {
                write!(f, "query \"{target}\" is not implemented yet")
            }
            Self::CreateLogDir { path, .. } => {
                write!(f, "failed to create log directory \"{}\"", path.display())
            }
            Self::OpenLogFile { path, .. } => {
                write!(f, "failed to open log file \"{}\"", path.display())
            }
            Self::InitLogger(_) => f.write_str("failed to initialize logging"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CurrentDir(source)
            | Self::MakeAbsolute { source, .. }
            | Self::CreateLogDir { source, .. }
            | Self::OpenLogFile { source, .. } => Some(source),
            Self::InitLogger(source) => Some(source.as_ref()),
            Self::HomeNotSet { .. }
            | Self::UnsupportedTilde { .. }
            | Self::NoParentDirectory { .. }
            | Self::QueryNotImplemented { .. } => None,
        }
    }
}
