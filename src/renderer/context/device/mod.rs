mod logical;
mod physical;

const DEVICE_EXTENSIONS: &[*const std::ffi::c_char] = &crate::helpers::as_const_pointers([
    ash::ext::image_drm_format_modifier::NAME,
    ash::ext::external_memory_dma_buf::NAME,
    ash::khr::external_memory_fd::NAME,
    ash::khr::external_semaphore_fd::NAME,
    ash::ext::queue_family_foreign::NAME,
]);

trait GPUDevice {}

pub(super) struct Device {
    physical: physical::PhysicalDevice,
    logical: Option<logical::LogicalDevice>,
}

impl Device {
    fn new(physical: physical::PhysicalDevice) -> Self {
        Self {
            physical,
            logical: None,
        }
    }

    fn get_physical(&self) -> &physical::PhysicalDevice {
        &self.physical
    }

    fn get_logical(&self) -> Option<&logical::LogicalDevice> {
        self.logical.as_ref()
    }

    fn get_logical_mut(&mut self) -> Option<&mut logical::LogicalDevice> {
        self.logical.as_mut()
    }

    fn activate(
        &mut self,
        instance: &ash::Instance,
    ) -> Result<&logical::LogicalDevice, Box<dyn std::error::Error>> {
        let logical = match self.logical.take() {
            Some(logical) => logical,
            None => create_logical(&self.physical, instance)?,
        };

        Ok(self.logical.insert(logical))
    }

    fn activate_mut(
        &mut self,
        instance: &ash::Instance,
    ) -> Result<&mut logical::LogicalDevice, Box<dyn std::error::Error>> {
        let logical = match self.logical.take() {
            Some(logical) => logical,
            None => create_logical(&self.physical, instance)?,
        };

        Ok(self.logical.insert(logical))
    }

    fn deactivate(&mut self) {
        self.logical = None;
    }
}
fn create_logical(
    physical: &physical::PhysicalDevice,
    instance: &ash::Instance,
) -> Result<logical::LogicalDevice, Box<dyn std::error::Error>> {
    let queue_priorities = [1.0f32];
    let queue_create_infos = if physical.graphics_family_index == physical.transfer_family_index {
        vec![
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical.graphics_family_index)
                .queue_priorities(&queue_priorities),
        ]
    } else {
        vec![
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical.graphics_family_index)
                .queue_priorities(&queue_priorities),
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(physical.transfer_family_index)
                .queue_priorities(&queue_priorities),
        ]
    };

    let mut vulkan_12_features =
        ash::vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
    let mut vulkan_13_features =
        ash::vk::PhysicalDeviceVulkan13Features::default().dynamic_rendering(true);

    let device_create_info = ash::vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(DEVICE_EXTENSIONS)
        .push_next(&mut vulkan_12_features)
        .push_next(&mut vulkan_13_features);

    let inner = unsafe { instance.create_device(physical.inner, &device_create_info, None)? };

    let graphics_queue = unsafe { inner.get_device_queue(physical.graphics_family_index, 0) };
    let transfer_queue = unsafe { inner.get_device_queue(physical.transfer_family_index, 0) };

    Ok(logical::LogicalDevice {
        inner,
        graphics_queue,
        transfer_queue,
    })
}

fn discover(instance: &ash::Instance) -> Result<Vec<Device>, Box<dyn std::error::Error>> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    todo!()
}
