mod instance;

const API_VERSION: u32 = ash::vk::API_VERSION_1_3;
const APP_VERSION_VK: u32 = ash::vk::make_api_version(
    0,
    crate::APP_VERSION.major,
    crate::APP_VERSION.minor,
    crate::APP_VERSION.patch,
);

#[cfg(debug_assertions)]
const INSTANCE_LAYERS: &[*const std::ffi::c_char] = &[c"VK_LAYER_KHRONOS_validation"
    .as_ptr()
    .cast::<std::ffi::c_char>()];
#[cfg(not(debug_assertions))]
const INSTANCE_LAYERS: &[*const std::ffi::c_char] = &[];

const INSTANCE_EXTENSIONS: &[*const std::ffi::c_char] = &[
    ash::khr::external_memory_capabilities::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::khr::external_semaphore_capabilities::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::khr::get_physical_device_properties2::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
];

const DEVICE_EXTENSIONS: &[*const std::ffi::c_char] = &[
    ash::ext::image_drm_format_modifier::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::ext::external_memory_dma_buf::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::khr::external_memory_fd::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::khr::external_semaphore_fd::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
    ash::ext::queue_family_foreign::NAME
        .as_ptr()
        .cast::<std::ffi::c_char>(),
];

pub(super) struct Context {
    pub device: ash::Device,
    pub physical_device: ash::vk::PhysicalDevice,
    pub instance: instance::Instance,
    pub entry: ash::Entry,
}

impl Context {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { ash::Entry::load()? };
        let instance = instance::Instance::new(&entry)?;

        todo!()
    }
}
