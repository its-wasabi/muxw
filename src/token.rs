// TODO: Think about balancing enum size like move bigger types to the Box<T>,

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Seat(crate::backend::seat::SeatEvent),

    Libinput,
    Input(crate::backend::input::InputEvent),

    WaylandSocket,
    WaylandDisplay,
    WaylandClientDisconnected(wayland_server::backend::ClientId),

    // TODO: Think if it shouldn't be some DrmEvent enum that can be either card or udev
    DrmUdev,
    // DrmCard(crate::backend::drm::DrmCardKey),
    Config(crate::config::ConfigRequest),

    Shutdown,
}
