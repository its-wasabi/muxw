// TODO: Think about using references if Token needs to reference bigger blob of data e.g.: if you
// introduce Token::Notify variant it would probably carry notify data which eventually will be
// String you could potentially pass ownership of that string and string itself is mainly size of
// the pointer but its already making it not optimal for events that carry only its variant in the
// Token... Think about it (hell or low performance) (making data indirect two times) Token ->
// NotifyData(String) -> String in heap is also not good
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    SeatEvent,
    SeatEnable,
    SeatDisable,
    SeatOpenRequest(Box<SeatOpenData>),
    SeatCloseRequest(std::os::fd::RawFd),

    WaylandSocket,
    WaylandDisplay,
    WaylandClientDisconnected(wayland_server::backend::ClientId),

    DrmUdev,
    DrmCard(crate::backend::drm::DrmCardKey),

    Input(crate::backend::input::InputEvent),
    Config(crate::config::ConfigRequest),
}

#[derive(Debug, Clone)]
pub struct SeatOpenData {
    pub path: std::path::PathBuf,
    pub reply: crossbeam_channel::Sender<Result<std::os::fd::OwnedFd, i32>>,
}

impl PartialEq for SeatOpenData {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
