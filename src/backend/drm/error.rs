#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),

    NotPrimaryCard,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPrimaryCard => write!(f, "device is not a primary DRM card",),
            Self::Io(err) => write!(f, "drm device io error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
