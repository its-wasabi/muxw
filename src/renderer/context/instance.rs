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
            .enabled_layer_names(super::INSTANCE_LAYERS)
            .enabled_extension_names(&super::INSTANCE_EXTENSIONS)
            .flags(create_flags);

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(Self { inner: instance })
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.inner.destroy_instance(None) };
    }
}
