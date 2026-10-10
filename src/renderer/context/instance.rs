#[cfg(debug_assertions)]
const INSTANCE_LAYERS: &[*const std::ffi::c_char] =
    &crate::helpers::as_const_pointers([c"VK_LAYER_KHRONOS_validation"]);
#[cfg(not(debug_assertions))]
const INSTANCE_LAYERS: &[*const std::ffi::c_char] = &crate::helpers::as_const_pointers([]);

const INSTANCE_EXTENSIONS: &[*const std::ffi::c_char] = &crate::helpers::as_const_pointers([
    ash::khr::external_memory_capabilities::NAME,
    ash::khr::external_semaphore_capabilities::NAME,
    ash::khr::get_physical_device_properties2::NAME,
]);

pub(super) struct Instance {
    inner: ash::Instance,
}

impl Instance {
    pub(super) fn new(entry: &ash::Entry) -> Result<Self, Box<dyn std::error::Error>> {
        let application_info = ash::vk::ApplicationInfo::default()
            .api_version(super::API_VERSION)
            .application_name(crate::APP_NAME_CSTR)
            .engine_version(super::APP_VERSION_VK)
            .application_version(super::APP_VERSION_VK)
            .engine_name(crate::APP_NAME_CSTR);
        let create_flags = ash::vk::InstanceCreateFlags::empty();
        let create_info = ash::vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&INSTANCE_LAYERS)
            .enabled_extension_names(INSTANCE_EXTENSIONS)
            .flags(create_flags);

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(Self { inner: instance })
    }
}

impl std::ops::Deref for Instance {
    type Target = ash::Instance;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.inner.destroy_instance(None) };
    }
}
