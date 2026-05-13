use crate::event_loop::event::EventFlags;
use std::os::fd::{AsRawFd, BorrowedFd, RawFd};

pub enum Source {
    Wayland(RawFd),
    Input(RawFd),
    Config(super::channel::Receiver<muxw_types::config::ConfigEvent>),
}

impl Source {
    pub(super) fn fd(&self) -> BorrowedFd<'_> {
        match self {
            // SAFETY: these RawFds are valid for the lifetime of the compositor
            Self::Wayland(fd) | Self::Input(fd) => unsafe {
                std::os::fd::BorrowedFd::borrow_raw(*fd)
            },
            Self::Config(rx) => rx.as_fd(),
        }
    }

    pub(super) fn options(&self) -> super::event::Options {
        match self {
            // Wayland socket: readable + edge-triggered
            Self::Wayland(_) => super::event::Options::default().readable().edge_triggered(),
            // libinput fd: same
            Self::Input(_) => super::event::Options::default().readable().edge_triggered(),
            // Config channel eventfd: readable + edge-triggered
            Self::Config(_) => super::event::Options::default().readable().edge_triggered(),
        }
    }

    /// Drive the compositor state machine from a ready fd.
    ///
    /// This mirrors wayland-server's `Dispatch` model: the source knows which
    /// method on the state to call; the state owns the mutation logic.
    /// `flags` carries what epoll reported so handlers can react to errors/HUP.
    pub(super) fn dispatch(
        &self,
        compositor: &mut crate::compositor::Compositor,
        flags: EventFlags,
    ) {
        match self {
            Self::Wayland(_) => compositor.handle_wayland(),
            Self::Input(_) => compositor.handle_input(),
            Self::Config(rx) => {
                if let Err(e) = rx.drain(|event| compositor.handle_config_event()) {
                    here!("config channel drain error: {e}");
                }
            }
        }
    }
}
