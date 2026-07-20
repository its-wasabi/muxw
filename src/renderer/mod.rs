use ash::{ext, khr, vk};
use std::ffi::CStr;

pub struct Renderer {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub graphics_family_index: u32,
    pub command_pool: vk::CommandPool,
}

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };

        // 1. Instance Creation (No winit, no surface extensions needed!)
        let app_name = c"Wayland-Compositor";
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .api_version(vk::API_VERSION_1_3);

        let instance_create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe { entry.create_instance(&instance_create_info, None)? };

        // 2. Physical Device Selection
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let physical_device = physical_devices
            .into_iter()
            .find(|&pdevice| {
                // In a real compositor, you want to match this physical_device
                // to the exact DRM node (/dev/dri/cardX) the DrmManager opened
                // using the VK_EXT_physical_device_drm extension.
                let props = unsafe { instance.get_physical_device_properties(pdevice) };
                props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                    || props.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU
            })
            .ok_or("No suitable Vulkan physical device found")?;

        // 3. Queue Family Selection
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let graphics_family_index = queue_families
            .iter()
            .enumerate()
            .find(|(_, props)| props.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|(idx, _)| idx as u32)
            .ok_or("No graphics queue found")?;

        // 4. Logical Device Creation (Extensions for DMA-BUF and DRM)
        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_family_index)
            .queue_priorities(&queue_priorities);

        let device_extensions = [
            // Required to import Wayland client buffers (DMA-BUFs)
            khr::external_memory_fd::NAME.as_ptr(),
            ext::external_memory_dma_buf::NAME.as_ptr(),
            ext::image_drm_format_modifier::NAME.as_ptr(),
        ];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_extension_names(&device_extensions);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family_index, 0) };

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(graphics_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe { device.create_command_pool(&pool_create_info, None)? };

        tracing::info!("Vulkan Renderer initialized successfully.");

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            graphics_queue,
            graphics_family_index,
            command_pool,
        })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}
