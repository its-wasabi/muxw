mod device;
mod instance;

const API_VERSION: u32 = ash::vk::API_VERSION_1_3;
const APP_VERSION_VK: u32 = ash::vk::make_api_version(
    0,
    crate::APP_VERSION.major,
    crate::APP_VERSION.minor,
    crate::APP_VERSION.patch,
);

pub(super) struct Context {
    pub devices: Vec<device::Device>,

    pub instance: instance::Instance,
    pub entry: ash::Entry,
}

impl Context {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };
        let instance = instance::Instance::new(&entry)?;

        Ok(Self {
            entry,
            instance,
            devices: Vec::with_capacity(1),
        })
    }
}

// fn enumerate_devices(
//     instance: &ash::Instance,
// ) -> Result<Vec<device::Device>, Box<dyn std::error::Error>> {
//     let physical_devices = unsafe { instance.enumerate_physical_devices()? };
//
//     let physical = physical_devices
//         .into_iter()
//         .filter_map(|physical_device| {
//             let queue_family_properties =
//                 unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
//
//             let graphics_queue_family_index =
//                 find_queue(&queue_family_properties, ash::vk::QueueFlags::GRAPHICS);
//             let transfer_queue_family_index =
//                 find_queue(&queue_family_properties, ash::vk::QueueFlags::TRANSFER);
//
//             let (graphics_queue_family_index, transfer_queue_family_index) =
//                 match (graphics_queue_family_index, transfer_queue_family_index) {
//                     (None, _) => return None,
//
//                     (Some(g), None) => (g, g),
//                     (Some(g), Some(t)) => (g, t),
//                 };
//
//             let mut drm_properties = ash::vk::PhysicalDeviceDrmPropertiesEXT::default();
//             let mut properties_2 =
//                 ash::vk::PhysicalDeviceProperties2::default().push_next(&mut drm_properties);
//             unsafe {
//                 instance.get_physical_device_properties2(physical_device, &mut properties_2);
//             }
//
//             if drm_properties.has_primary == 0 || drm_properties.has_render == 0 {
//                 return None;
//             }
//         })
//         .collect::<Vec<_>>();
//
//     Ok(physical)
// }
