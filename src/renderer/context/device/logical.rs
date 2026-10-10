pub(super) struct LogicalDevice {
    pub inner: ash::Device,
    pub graphics_queue: ash::vk::Queue,
    pub transfer_queue: ash::vk::Queue,
}

impl std::ops::Deref for LogicalDevice {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
