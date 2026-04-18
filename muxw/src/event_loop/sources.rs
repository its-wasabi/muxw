use std::os::fd::RawFd;

// bitflags::bitflags! {
//     pub struct Source: u32 {
//     const Wayland(RawFd),
//     const Input(RawFd),
//     const Config(std::sync::mpsc::Receiver<Box<dyn muxw_types::config::ConfigEvent>>),
//
//     }
// }

pub enum Source {
    Wayland(RawFd),
    Input(RawFd),
    Config(std::sync::mpsc::Receiver<muxw_types::config::ConfigEvent>),
}

impl Source {
    // TODO: Check if BorrowedFd is the best type to be used here
    pub(super) fn fd(&self) -> std::os::fd::BorrowedFd<'_> {
        match self {
            Source::Wayland(fd) => todo!("Extract fd"),
            Source::Input(fd) => todo!("Extract fd"),
            Source::Config(receiver) => todo!("Create some indirection layer that allows for fds"),
        }
    }

    pub(super) fn options(&self) -> super::event::Options {
        match self {
            Source::Wayland(_) => todo!(),
            Source::Input(_) => todo!(),
            Source::Config(receiver) => todo!(),
        }
    }
}
