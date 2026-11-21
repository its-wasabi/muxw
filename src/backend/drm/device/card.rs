use std::mem::ManuallyDrop;
use std::os::fd::{AsFd, AsRawFd};

use drm::Device as DRMDevice;
use drm::control::Device as DRMControlDevice;

const DRM_MAJOR: u32 = 226;
const MAX_PRIMARY_MINOR: u32 = 64;

pub(super) struct Card {
    device: crate::backend::session::Device,
    seat_handle: crate::backend::session::SeatHandle,
}

impl Card {
    pub(super) fn new(
        devnum: u64,
        devnode: &std::path::Path,
        seat_handle: crate::backend::session::SeatHandle,
    ) -> Result<Self, crate::backend::drm::error::Error> {
        if !is_primary_card(devnum) {
            return Err(crate::backend::drm::error::Error::NotPrimaryCard);
        }

        let card = Self {
            device: seat_handle.open_device(devnode)?,
            seat_handle,
        };

        card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)?;
        card.set_client_capability(drm::ClientCapability::Atomic, true)?;

        Ok(card)
    }
}

impl std::os::fd::AsFd for Card {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.device.fd.as_fd()
    }
}

impl drm::Device for Card {}
impl drm::control::Device for Card {}

impl Drop for Card {
    fn drop(&mut self) {
        self.seat_handle.close_device(self.device.id);
    }
}

#[inline]
fn is_primary_card(devnum: u64) -> bool {
    rustix::fs::major(devnum) == DRM_MAJOR && rustix::fs::minor(devnum) < MAX_PRIMARY_MINOR
}
