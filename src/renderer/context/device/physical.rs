#[derive(Clone, Copy)]
pub(super) struct PhysicalDevice {
    pub inner: ash::vk::PhysicalDevice,
    pub properties: ash::vk::PhysicalDeviceProperties,
    pub memory_properties: ash::vk::PhysicalDeviceMemoryProperties,
    pub primary_node: (crate::helpers::Major, crate::helpers::Minor),
    pub render_node: (crate::helpers::Major, crate::helpers::Minor),
    pub graphics_family_index: u32,
    pub transfer_family_index: u32,
}

impl PhysicalDevice {
    fn new(
        instance: &ash::Instance,
        physical_device: ash::vk::PhysicalDevice,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let extension_properties =
            unsafe { instance.enumerate_device_extension_properties(physical_device)? };

        'required: for &required in super::DEVICE_EXTENSIONS {
            let required = unsafe { std::ffi::CStr::from_ptr(required) };

            for properties in &extension_properties {
                if properties.extension_name_as_c_str()? == required {
                    continue 'required;
                }
            }

            return Ok(None);
        }

        todo!()
    }
}

impl std::ops::Deref for PhysicalDevice {
    type Target = ash::vk::PhysicalDevice;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn find_queue(
    queue_family_properties: &[ash::vk::QueueFamilyProperties],
    flag: ash::vk::QueueFlags,
) -> Option<usize> {
    queue_family_properties
        .iter()
        .position(|queue| queue.queue_flags.contains(flag))
}
