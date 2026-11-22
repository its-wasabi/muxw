mod card;

use drm::Device as DRMDevice;
use drm::control::Device as DRMControlDevice;
use eyre::Context;
use std::os::fd::AsFd;

const DRM_MAJOR: u32 = 226;
const MAX_PRIMARY_MINOR: u32 = 64;

slotmap::new_key_type! {
    pub struct DeviceKey;
}

pub(super) struct Device {
    card: card::Card,
    handle: crate::event_loop::IoHandle<crate::Event>,

    devnum: u64,
    resources: drm::control::ResourceHandles,
}

impl Device {
    pub(super) fn new(
        device: &udev::Device,
        seat_handle: crate::backend::session::SeatHandle,
        key: DeviceKey,
        event_loop: &mut crate::event_loop::EventLoop<crate::Event>,
    ) -> eyre::Result<Option<Self>> {
        let (Some(devnum), Some(devnode)) = (device.devnum(), device.devnode()) else {
            return Ok(None);
        };
        let Some(card) = card::Card::new(devnum, devnode, seat_handle)? else {
            return Ok(None);
        };

        let handle = event_loop
            .register_fd(
                &card.as_fd(),
                crate::event_loop::Interest::Readable,
                super::Event::Device(key),
            )
            .wrap_err_with(|| {
                format!(
                    "failed to register DRM fd for {} in event loop",
                    devnode.display()
                )
            })?;

        let resources = card.resource_handles().wrap_err_with(|| {
            format!(
                "failed to query DRM resource handles on {}",
                devnode.display()
            )
        })?;

        for &connector_handle in &resources.connectors {
            let connector = card
                .get_connector(connector_handle, false)
                .wrap_err_with(|| {
                    format!(
                        "failed to query connector {:?} on {}",
                        connector_handle,
                        devnode.display()
                    )
                })?;

            if connector.state() == drm::control::connector::State::Connected {
                tracing::trace!("{:?} Connected", connector.interface());
            }
        }

        Ok(Some(Self {
            card,
            handle,
            devnum,
            resources,
        }))
    }

    pub(super) const fn resume(&mut self) -> eyre::Result<()> {
        todo!()
    }

    #[inline]
    pub(super) const fn devnum(&self) -> u64 {
        self.devnum
    }
}

struct Card(std::fs::File);

impl Card {
    fn new(path: &std::path::Path) -> eyre::Result<Self> {
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
