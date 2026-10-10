// NOTE: Used to notify if Token grows too big
const _: () = assert!(std::mem::size_of::<Token>() <= 48);

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Seat(crate::backend::seat::SeatEvent),
    Input(crate::backend::input::InputEvent),
    Drm(crate::backend::drm::Event),
    Wayland(crate::wayland::Event),

    Config(crate::config::ConfigRequest),

    Exit,
}

impl From<crate::backend::seat::SeatEvent> for Token {
    fn from(value: crate::backend::seat::SeatEvent) -> Self {
        Self::Seat(value)
    }
}

impl From<crate::backend::input::InputEvent> for Token {
    fn from(value: crate::backend::input::InputEvent) -> Self {
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
