#[derive(Debug)]
pub enum Error {
    CreateDisplay(wayland_server::backend::InitError),
    BindSocket {
        name: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    BindAutoSocket(Box<dyn std::error::Error + Send + Sync>),
    RegisterSocket(std::io::Error),
    RegisterDisplay(std::io::Error),
    AcceptClient(std::io::Error),
    InsertClient(std::io::Error),
    DispatchClients(std::io::Error),
    FlushClients(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateDisplay(_) => f.write_str("failed to create Wayland display"),
            Self::BindSocket { name, .. } => {
                write!(f, "failed to bind Wayland socket \"{name}\"")
            }
            Self::BindAutoSocket(_) => f.write_str("failed to find a free Wayland socket"),
            Self::RegisterSocket(_) => {
                f.write_str("failed to register Wayland socket with the event loop")
            }
            Self::RegisterDisplay(_) => {
                f.write_str("failed to register Wayland display with the event loop")
            }
            Self::AcceptClient(_) => f.write_str("failed to accept Wayland client connection"),
            Self::InsertClient(_) => f.write_str("failed to register Wayland client"),
            Self::DispatchClients(_) => f.write_str("failed to dispatch Wayland clients"),
            Self::FlushClients(_) => f.write_str("failed to flush Wayland clients"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateDisplay(source) => Some(source),

            Self::BindSocket { source, .. } | Self::BindAutoSocket(source) => Some(source.as_ref()),

            Self::RegisterSocket(source)
            | Self::RegisterDisplay(source)
            | Self::AcceptClient(source)
            | Self::InsertClient(source)
            | Self::DispatchClients(source)
            | Self::FlushClients(source) => Some(source),
        }
    }
}
