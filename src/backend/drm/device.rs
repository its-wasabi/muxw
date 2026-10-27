use std::os::fd::AsFd;

const MAX_PRIMARY_MINOR: u32 = 64;

pub(super) struct Device {
    file: std::fs::File,
    devnum: u64,
    handle: crate::event_loop::IoHandle<crate::token::Token>,
    outputs: Vec<super::OutputKey>,
}

impl Device {
    // TODO: Replace all None with correct errors
    pub(super) fn register_self(
        event_loop: &mut crate::event_loop::EventLoop,
        device_list: &mut slotmap::SlotMap<super::DeviceKey, Self>,
        device: &udev::Device,
    ) -> Result<Option<super::DeviceKey>, Box<dyn std::error::Error>> {
        let (Some(devnum), Some(devnode)) = (device.devnum(), device.devnode()) else {
            return Ok(None);
        };

        // Ignore `renderer` files
        if rustix::fs::minor(devnum) < MAX_PRIMARY_MINOR {
            return Ok(None);
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(devnode)?;

        Ok(Some(device_list.try_insert_with_key(|key| {
            let handle = event_loop.register_fd(
                &file,
                crate::event_loop::Interest::Readable,
                // polling::PollMode::Edge,
                super::Event::Device(key),
            )?;

            Ok::<Self, Box<dyn std::error::Error>>(Self {
                file,
                devnum,
                handle,
                outputs: Vec::new(),
            })
        })?))
    }

    pub(super) fn release_master(&self) -> std::io::Result<()> {
        drm::Device::release_master_lock(self)
    }

    pub(super) fn query_outputs(&self) -> Vec<Output> {
        todo!()
    }
}

impl AsFd for Device {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl drm::Device for Device {}
impl drm::control::Device for Device {}

pub struct Output {
    device: super::DeviceKey,

    connector: drm::control::connector::Handle,
    crtc: drm::control::crtc::Handle,
    mode: drm::control::Mode,
}
