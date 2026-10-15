use std::os::{fd::AsFd, unix::fs::OpenOptionsExt};

const DRM_MAJOR: u32 = 226;
const MINORS_PER_NODE_TYPE: u32 = 64;

struct DeviceMap {
    active_mask: u64,
    items: Vec<Device>,
}

impl DeviceMap {
    fn new() -> Self {
        Self {
            active_mask: 0,
            items: Vec::with_capacity(super::DEVICES_DEFAULT_CAPACITY),
        }
    }

    #[inline]
    fn get_minor(devnum: u64) -> usize {
        rustix::fs::minor(devnum) as usize
    }

    pub fn get(&self, devnum: u64) -> Option<&Device> {
        let minor = Self::get_minor(devnum);
        if minor >= 64 || (self.active_mask & (1_u64 << minor)) == 0 {
            return None;
        }
        let mask = (1_u64 << minor) - 1;
        let dense_idx = (self.active_mask & mask).count_ones() as usize;
        self.items.get(dense_idx)
    }

    pub fn get_mut(&mut self, devnum: u64) -> Option<&mut Device> {
        let minor = Self::get_minor(devnum);
        if minor >= 64 || (self.active_mask & (1_u64 << minor)) == 0 {
            return None;
        }
        let mask = (1_u64 << minor) - 1;
        let dense_idx = (self.active_mask & mask).count_ones() as usize;
        self.items.get_mut(dense_idx)
    }

    pub fn insert(&mut self, device: Device) -> Result<(), Device> {
        let minor = Self::get_minor(device.devnum);
        if minor >= 64 {
            return Err(device);
        }
        let bit = 1_u64 << minor;
        if (self.active_mask & bit) != 0 {
            return Err(device); // Slot already taken
        }
        let mask = bit - 1;
        let dense_idx = (self.active_mask & mask).count_ones() as usize;
        self.items.insert(dense_idx, device);
        self.active_mask |= bit;
        Ok(())
    }

    pub fn remove(&mut self, devnum: u64) -> Option<Device> {
        let minor = Self::get_minor(devnum);
        if minor >= 64 {
            return None;
        }
        let bit = 1_u64 << minor;
        if (self.active_mask & bit) == 0 {
            return None;
        }
        let mask = bit - 1;
        let dense_idx = (self.active_mask & mask).count_ones() as usize;
        let device = self.items.remove(dense_idx);
        self.active_mask ^= bit;
        Some(device)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Device> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Device> {
        self.items.iter_mut()
    }
}

pub(super) struct Device {
    file: std::fs::File,
    devnum: u64,
}

impl Device {
    // TODO: Replace all with correct errors
    pub(super) fn new(
        event_loop: &crate::event_loop::EventLoop,
        device: &udev::Device,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let (Some(devnum), Some(devnode)) = (device.devnum(), device.devnode()) else {
            return Ok(None);
        };

        // Filter out non card files
        {
            if !(rustix::fs::major(devnum) == DRM_MAJOR
                && rustix::fs::minor(devnum) < MINORS_PER_NODE_TYPE)
            {
                return Ok(None);
            }
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(devnode)?;

        Ok(Some(Self { file, devnum }))
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

pub struct Output {}
