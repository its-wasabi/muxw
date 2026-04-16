/// When other threads require something from the config

pub enum ConfigCommand {
    Reload,
}

/// When config requires other threads to do something
pub trait ConfigEvent: Send + 'static {}
