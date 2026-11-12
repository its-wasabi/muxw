use drm::Device as DRMDevice;
use drm::control::Device as DRMControlDevice;
use std::os::fd::AsFd;

const DRM_MAJOR: u32 = 226;
const MAX_PRIMARY_MINOR: u32 = 64;

slotmap::new_key_type! {
    pub struct DeviceKey;
}

pub(super) struct Device {
    pub(super) file: std::fs::File,
    pub(super) devnum: u64,

    outputs: Vec<super::OutputKey>,
}

impl Device {
    pub(super) fn new(device: &udev::Device) -> Result<Self, super::error::Error> {
        if let Some(pci) = device.parent_with_subsystem("pci")? {
            let vendor = pci
                .property_value("ID_VENDOR_FROM_DATABASE")
                .and_then(|s| s.to_str());
            let model = pci
                .property_value("ID_MODEL_FROM_DATABASE")
                .and_then(|s| s.to_str());
            let is_boot_vga = pci.attribute_value("boot_vga").and_then(|s| s.to_str()) == Some("1");

            tracing::info!("Drm device {vendor:?} | {model:?} | {is_boot_vga:?}");
        }

        let (Some(devnum), Some(devnode)) = (device.devnum(), device.devnode()) else {
            return Err(super::error::Error::NotPrimaryCard);
        };

        if !Self::is_primary_card(devnum) {
            return Err(super::error::Error::NotPrimaryCard);
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(devnode)?;

        let device = Self {
            file,
            devnum,
            outputs: Vec::new(),
        };

        // TODO: map errors
        device.set_client_capability(drm::ClientCapability::UniversalPlanes, true)?;
        device.set_client_capability(drm::ClientCapability::Atomic, true)?;

        Ok(device)
    }

    #[inline]
    pub(super) fn release_master(&self) -> std::io::Result<()> {
        drm::Device::release_master_lock(self)
    }

    #[inline]
    fn is_primary_card(devnum: u64) -> bool {
        rustix::fs::major(devnum) == DRM_MAJOR && rustix::fs::minor(devnum) < MAX_PRIMARY_MINOR
    }
}

impl AsFd for Device {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl drm::Device for Device {}
impl drm::control::Device for Device {}

impl Drop for Device {
    fn drop(&mut self) {}
}
