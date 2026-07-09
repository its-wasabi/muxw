// TODO: Handle card dynamically
// 2. Best card selection (not best but make use of all cards that have monitor connected)
// 3. Config API for card selection

// IMPORTANT: Hold event_loop key to deregister event source when removing device
// IMPORTANT: Merge monitor and enumerator device registration logic (mostly is_primary check)
// IMPORTANT: Also error handling here is non-existent if device cant be opened instead of
// compositor crash it should be simply ignored

use drm::Device;
use std::os::fd::AsFd;

const DEVICES_DEFAULT_CAPACITY: usize = 1;

pub struct DrmManager {
    cards: slab::Slab<DrmCard>,
    monitor: udev::MonitorSocket,

    device_keys: std::collections::HashMap<u64, DrmCardKey>,
}

impl DrmManager {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let monitor = udev::MonitorBuilder::new()?
            .match_subsystem("drm")?
            .listen()?;
        event_loop.register_source(
            &monitor.as_fd(),
            polling::PollMode::Edge,
            crate::token::Token::DrmUdev,
        )?;

        let mut drm_manager = Self {
            cards: slab::Slab::with_capacity(DEVICES_DEFAULT_CAPACITY),
            monitor,

            device_keys: std::collections::HashMap::with_capacity(DEVICES_DEFAULT_CAPACITY),
        };

        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("drm");
        for drm_device in enumerator.scan_devices()? {
            let sysname = drm_device.sysname().to_string_lossy();
            if !sysname.starts_with("card") || sysname.contains('-') {
                continue;
            }

            if let (Some(path), Some(devnum)) = (drm_device.devnode(), drm_device.devnum()) {
                let key = drm_manager.register_card(path, event_loop)?;
                drm_manager.device_keys.insert(devnum, key);
            }
        }

        Ok(drm_manager)
    }

    pub fn register_card(
        &mut self,
        path: &std::path::Path,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<DrmCardKey, Box<dyn std::error::Error>> {
        Self::reggister_card_inner(&mut self.cards, path, event_loop)
    }

    fn reggister_card_inner(
        cards: &mut slab::Slab<DrmCard>,
        path: &std::path::Path,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<DrmCardKey, Box<dyn std::error::Error>> {
        // let card = DrmCard::new(path)?;
        let vacant_entry = cards.vacant_entry();
        let key = DrmCardKey(vacant_entry.key());
        // event_loop.register_source(
        //     &card.as_fd(),
        //     polling::PollMode::Edge,
        //     crate::Token::DrmCard(key),
        // )?;
        // vacant_entry.insert(card);
        Ok(key)
    }

    pub fn deregister_card(&mut self, key: DrmCardKey) {
        Self::deregister_card_inner(&mut self.cards, key);
    }

    fn deregister_card_inner(cards: &mut slab::Slab<DrmCard>, key: DrmCardKey) {
        cards.remove(key.get());
    }

    pub fn dispatch_udev(
        &mut self,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) {
        for event in self.monitor.iter() {
            let Some(devnum) = event.devnum() else {
                unimplemented!("Log here");
                continue;
            };

            if rustix::fs::minor(devnum) > 63 {
                println!("Skipping (NOT A CARD NODE)");
                continue;
            }

            match event.event_type() {
                udev::EventType::Add => {
                    if let Some(path) = event.devnode()
                        && let Ok(key) =
                            Self::reggister_card_inner(&mut self.cards, path, event_loop)
                    {
                        self.device_keys.insert(devnum, key);
                    }
                }

                udev::EventType::Change => {
                    if let Some(key) = self.device_keys.get(&devnum) {
                        println!("Device connector state changed for card {}", key.get());
                        // TODO: Probing logic
                    }
                }

                udev::EventType::Remove => {
                    if let Some(key) = self.device_keys.remove(&devnum) {
                        Self::deregister_card_inner(&mut self.cards, key);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn dispatch_card(&mut self, key: DrmCardKey) {
        todo!("Hello");
    }
}

struct DrmCard {
    file: std::fs::File,
    devnum: u64,
    event_loop_key: crate::event_loop::IoKey,
}

impl DrmCard {
    fn new(
        path: &std::path::Path,
        devnum: u64,
        ev_key: crate::event_loop::IoKey,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let card = Self {
            file: std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)?,
            devnum,

            // TODO: Thinking about making event_loop registration part of Self::new foo
            event_loop_key: ev_key,
        };

        // card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)?;
        // card.set_client_capability(drm::ClientCapability::Atomic, true)?;

        Ok(card)
    }
}

impl std::os::fd::AsFd for DrmCard {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl drm::Device for DrmCard {}
impl drm::control::Device for DrmCard {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrmCardKey(usize);

impl DrmCardKey {
    pub const fn get(self) -> usize {
        self.0
    }
}
