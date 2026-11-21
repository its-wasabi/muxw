// NOTE: Used to notify if Token grows too big
const _: () = assert!(std::mem::size_of::<Token>() <= 48);

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Seat(crate::backend::session::Event),
    Input(crate::backend::input::Event),
    Drm(crate::backend::drm::Event),
    Wayland(crate::wayland::Event),
    Config(crate::config::Event),

    Exit,
}

impl From<crate::backend::session::Event> for Token {
    fn from(value: crate::backend::session::Event) -> Self {
        Self::Seat(value)
    }
}

impl From<crate::backend::input::Event> for Token {
    fn from(value: crate::backend::input::Event) -> Self {
        Self::Input(value)
    }
}

impl From<crate::backend::drm::Event> for Token {
    fn from(value: crate::backend::drm::Event) -> Self {
        Self::Drm(value)
    }
}

impl From<crate::wayland::Event> for Token {
    fn from(value: crate::wayland::Event) -> Self {
        Self::Wayland(value)
    }
}

impl From<crate::config::Event> for Token {
    fn from(value: crate::config::Event) -> Self {
        Self::Config(value)
    }
}
