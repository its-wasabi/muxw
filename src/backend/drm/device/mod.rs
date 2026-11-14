mod card;

use drm::Device as DRMDevice;
use drm::control::Device as DRMControlDevice;
use std::os::fd::AsFd;

const DRM_MAJOR: u32 = 226;
const MAX_PRIMARY_MINOR: u32 = 64;

slotmap::new_key_type! {
    pub struct DeviceKey;
}

pub(super) struct Device {
    card: card::Card,
    handle: crate::event_loop::IoHandle<crate::token::Token>,
    devnum: u64,
    outputs: Vec<super::OutputKey>,
}

impl Device {
    pub(super) fn new(
        device: &udev::Device,
        seat_handle: crate::backend::seat::SeatHandle,
        key: DeviceKey,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, super::error::Error> {
        let (Some(devnum), Some(devnode)) = (device.devnum(), device.devnode()) else {
            return Err(super::error::Error::NotPrimaryCard);
        };

        let card = card::Card::new(devnum, devnode, seat_handle)?;

        let handle = event_loop.register_fd(
            &card.as_fd(),
            crate::event_loop::Interest::Readable,
            super::Event::Device(key),
        )?;

        Ok(Self {
            card,
            handle,
            devnum,
            outputs: Vec::new(),
        })
    }

    #[inline]
    pub(super) fn devnum(&self) -> u64 {
        self.devnum
    }

    pub(super) fn release_master_lock(&self) -> Result<(), super::error::Error> {
        Ok(self.card.release_master_lock()?)
    }
}

struct Card(std::fs::File);

impl Card {
    fn new(path: &std::path::Path) -> Result<Self, super::error::Error> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .open(path)?;

        let card = Self(file);

        card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)?;
        card.set_client_capability(drm::ClientCapability::Atomic, true)?;

        Ok(card)
    }
}

impl AsFd for Card {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl drm::control::Device for Card {}

impl Drop for Card {
    fn drop(&mut self) {
        self.release_master_lock();
    }
}

#[inline]
fn is_primary_card(devnum: u64) -> bool {
    rustix::fs::major(devnum) == DRM_MAJOR && rustix::fs::minor(devnum) < MAX_PRIMARY_MINOR
}
