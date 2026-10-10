pub struct Report<'a>(pub &'a (dyn std::error::Error + 'static));

impl std::fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        let mut source = self.0.source();
        while let Some(error) = source {
            write!(f, ": {error}")?;
            source = error.source();
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    // TODO: Reorder them
    Config(crate::config::error::Error),
    Context(crate::context::error::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Config(error) => std::fmt::Display::fmt(error, f),
            Error::Context(error) => std::fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Config(error) => error.source(),
            Error::Context(error) => error.source(),
        }
    }
}

impl From<crate::config::error::Error> for Error {
    fn from(value: crate::config::error::Error) -> Self {
        Self::Config(value)
    }
}

impl From<crate::context::error::Error> for Error {
    fn from(value: crate::context::error::Error) -> Self {
        Self::Context(value)
    }
}
