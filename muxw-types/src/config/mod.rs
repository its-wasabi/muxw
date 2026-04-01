/// When other threads require something from the config
pub trait ConfigCommand {}

/// When config requires other threads to do something
pub trait ConfigEvent {}
