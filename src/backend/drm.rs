// TODO: Handle card dynamically
// 2. Best card selection (not best but make use of all cards that have monitor connected)
// 3. Config API for card selection

// IMPORTANT: For reference and valid usage read your older code on gitea cause this shit is broken
// vibecoded scum and should not be used for other purpose than high level overview

// IMPORTANT: Hold event_loop key to deregister event source when removing device
// IMPORTANT: Merge monitor and enumerator device registration logic (mostly is_primary check)
// IMPORTANT: Also error handling here is non-existent if device cant be opened instead of
// compositor crash it should be simply ignored

use std::os::fd::AsFd;

const DEVICES_DEFAULT_CAPACITY: usize = 4;

pub struct DrmManager {
    cards: slab::Slab<DrmCard>,
    monitor: udev::MonitorSocket,
    device_keys: std::collections::HashMap<u64, DrmCardKey>,
}

impl DrmManager {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
        renderer: &crate::renderer::Renderer,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!("DRM START");

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

        // Enumerate existing devices
        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("drm")?;

        for drm_device in enumerator.scan_devices()? {
            // Pass renderer down
            drm_manager.process_udev_add(drm_device, event_loop, renderer);
        }

        Ok(drm_manager)
    }

    /// Handles an ADD event from either the initial enumerator or udev monitor
    fn process_udev_add(
        &mut self,
        device: udev::Device,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
        renderer: &crate::renderer::Renderer,
    ) {
        let sysname = device.sysname().to_string_lossy();
        if !sysname.starts_with("card") || sysname.contains('-') {
            return;
        }

        if let (Some(path), Some(devnum)) = (device.devnode(), device.devnum()) {
            if rustix::fs::minor(devnum) > 63 {
                return;
            }

            match self.register_card_inner(path, devnum, event_loop) {
                Ok(key) => {
                    tracing::info!("Registered DRM card: {:?}", path);
                    self.device_keys.insert(devnum, key);

                    if let Some(card) = self.cards.get_mut(key.get()) {
                        // IMPORTANT: Make sure this is init_surface!
                        if let Err(e) = init_surface(renderer, card) {
                            tracing::error!("Vulkan takeover failed: {}", e);
                        }
                    }
                }
                Err(e) => tracing::error!("Failed to register card: {}", e),
            }
        }
    }

    fn register_card_inner(
        &mut self,
        path: &std::path::Path,
        devnum: u64,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<DrmCardKey, Box<dyn std::error::Error>> {
        let vacant_entry = self.cards.vacant_entry();
        let key = DrmCardKey(vacant_entry.key());

        let mut card = DrmCard::new(path, devnum)?;

        // Register the card's FD to the event loop using the Token assigned to this card
        let io_key = event_loop.register_source(
            &card.as_fd(),
            polling::PollMode::Edge,
            crate::token::Token::DrmCard(key),
        )?;

        card.event_loop_key = Some(io_key);
        vacant_entry.insert(card);

        Ok(key)
    }

    fn deregister_card_inner(
        &mut self,
        key: DrmCardKey,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
    ) {
        // Use `remove` instead of `try_remove` if you are on an older version of slab,
        // otherwise `try_remove` is fine.
        if self.cards.contains(key.get()) {
            let card = self.cards.remove(key.get());
            if let Some(io_key) = card.event_loop_key {
                // Pass a reference to the card as the second argument (the `source`)
                event_loop.deregister_source(io_key, &card);
            }
            tracing::info!("Deregistered DRM card {}", key.get());
        }
    }

    pub fn dispatch_udev(
        &mut self,
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
        renderer: &crate::renderer::Renderer,
    ) {
        // Collect events into a vector to free the borrow on `self.monitor`
        let events: Vec<_> = self.monitor.iter().collect();

        for event in events {
            let Some(devnum) = event.devnum() else {
                continue;
            };

            match event.event_type() {
                udev::EventType::Add => {
                    self.process_udev_add(event.device(), event_loop, renderer);
                }
                udev::EventType::Change => {
                    if let Some(&key) = self.device_keys.get(&devnum) {
                        tracing::info!("Device connector state changed for card {}", key.get());
                        // TODO: Re-probe connectors via drm::control::connector
                    }
                }
                udev::EventType::Remove => {
                    if let Some(key) = self.device_keys.remove(&devnum) {
                        self.deregister_card_inner(key, event_loop);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn dispatch_card(&mut self, key: DrmCardKey) {
        if let Some(card) = self.cards.get(key.get()) {
            use drm::control::Device as _; // Import the trait to use its methods

            // Call it as a method on the card
            match card.receive_events() {
                Ok(events) => {
                    for event in events {
                        // tracing::debug!("DRM Event received: {:?}", event);
                        // Trigger rendering of the next frame here
                    }
                }
                Err(e) => tracing::error!("Error reading DRM events: {}", e),
            }
        }
    }
    pub fn paint_all(&mut self, renderer: &crate::renderer::Renderer, color: [f32; 4]) {
        // Add this line to verify the loop is alive and generating new colors
        tracing::trace!("Painting frame with color: {:?}", color);

        for (_, card) in self.cards.iter_mut() {
            if let Some(surface) = &card.surface {
                if let Err(e) = draw_color(renderer, surface, color) {
                    tracing::error!("Failed to draw frame: {}", e);
                }
            }
        }
    }
}

struct DrmCard {
    file: std::fs::File,
    devnum: u64,
    event_loop_key: Option<crate::event_loop::IoKey>,
    surface: Option<CardSurface>,
}

pub struct CardSurface {
    pub gbm_bo: gbm::BufferObject<()>,
    pub vk_image: ash::vk::Image,
    pub vk_memory: ash::vk::DeviceMemory,
    pub fb: drm::control::framebuffer::Handle, // <-- Add this
    pub crtc: drm::control::crtc::Handle,      // <-- Add this
}

impl DrmCard {
    fn new(path: &std::path::Path, devnum: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        let card = Self {
            file,
            devnum,
            event_loop_key: None, // Will be set when registered
            surface: None,
        };

        card.set_atomic_universal_planes(true)?;
        Ok(card)
    }

    fn set_atomic_universal_planes(&self, enable: bool) -> Result<(), Box<dyn std::error::Error>> {
        use drm::Device;
        self.set_client_capability(drm::ClientCapability::UniversalPlanes, enable)?;
        self.set_client_capability(drm::ClientCapability::Atomic, enable)?;
        Ok(())
    }
}

impl std::os::fd::AsFd for DrmCard {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl drm::Device for DrmCard {}
impl drm::control::Device for DrmCard {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrmCardKey(usize);

impl DrmCardKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

use ash::vk;

// A wrapper to force GBM memory into DRM 0.15.0
struct GbmToDrm {
    size: (u32, u32),
    pitch: u32,
    handle: drm::buffer::Handle,
    format: drm::buffer::DrmFourcc,
}

impl drm::buffer::Buffer for GbmToDrm {
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn format(&self) -> drm::buffer::DrmFourcc {
        self.format
    }
    fn pitch(&self) -> u32 {
        self.pitch
    }
    fn handle(&self) -> drm::buffer::Handle {
        self.handle
    }
}

pub fn init_surface(
    renderer: &crate::renderer::Renderer,
    card: &mut crate::backend::drm::DrmCard,
) -> Result<(), Box<dyn std::error::Error>> {
    use drm::control::Device as ControlDevice;

    let res_handles = card.resource_handles()?;

    // 1. Use filter_map with .ok() to silently drop connectors that fail to query
    let connector = match res_handles
        .connectors()
        .iter()
        .filter_map(|&conn| card.get_connector(conn, true).ok())
        .find(|c| c.state() == drm::control::connector::State::Connected)
    {
        Some(conn) => conn,
        None => {
            tracing::info!(
                "Skipping card (Devnum {}): No connected monitor",
                card.devnum
            );
            return Ok(());
        }
    };

    // 2. Safely check for available modes before indexing
    let modes = connector.modes();
    if modes.is_empty() {
        tracing::warn!(
            "Skipping card (Devnum {}): Monitor connected, but no modes available",
            card.devnum
        );
        return Ok(());
    }
    let mode = modes[0];

    // 3. Safely check for available CRTCs before indexing
    let crtcs = res_handles.crtcs();
    if crtcs.is_empty() {
        tracing::warn!(
            "Skipping card (Devnum {}): No CRTCs available on this device",
            card.devnum
        );
        return Ok(());
    }
    let crtc = crtcs[0];

    let (width, height) = mode.size();
    tracing::info!("Setting up Vulkan Surface for {}x{}", width, height);

    // ... continue with your GBM and Vulkan setup below ...

    // Safer GBM initialization
    let gbm = gbm::Device::new(&*card).map_err(|e| format!("GBM init failed: {}", e))?;
    let gbm_bo = gbm
        .create_buffer_object::<()>(
            width as u32,
            height as u32,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT
                | gbm::BufferObjectFlags::RENDERING
                | gbm::BufferObjectFlags::LINEAR,
        )
        .map_err(|e| format!("Failed to create GBM BO: {:?}", e))?;

    use std::os::fd::IntoRawFd; // Add this import at the top of the file

    let dma_buf_fd = gbm_bo
        .fd()
        .map_err(|e| format!("Failed to export DMA fd: {:?}", e))?;
    let raw_fd = dma_buf_fd.into_raw_fd(); // Consumes the object, keeps the FD open!

    // 1. Extract the raw GEM handle from the GBM C Union
    let raw_handle = unsafe { gbm_bo.handle().u32_ };
    let non_zero_handle =
        std::num::NonZeroU32::new(raw_handle).expect("GBM returned a null buffer handle");

    // 2. Wrap it in our custom struct that satisfies DRM 0.15.0
    let bridge_buffer = GbmToDrm {
        size: (width as u32, height as u32),
        pitch: gbm_bo.stride(),
        handle: drm::buffer::Handle::from(non_zero_handle),
        format: drm::buffer::DrmFourcc::Xrgb8888,
    };

    // 3. Hand it to DRM
    let fb = card.add_framebuffer(&bridge_buffer, 24, 32)?;
    card.set_crtc(crtc, Some(fb), (0, 0), &[connector.handle()], Some(mode))?;

    let mut external_memory_info = vk::ExternalMemoryImageCreateInfo::default()
        .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

    let image_create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::B8G8R8A8_UNORM)
        .extent(vk::Extent3D {
            width: width as u32,
            height: height as u32,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::LINEAR)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .push_next(&mut external_memory_info);

    let vk_image = unsafe { renderer.device.create_image(&image_create_info, None)? };
    let mem_reqs = unsafe { renderer.device.get_image_memory_requirements(vk_image) };
    let mem_props = unsafe {
        renderer
            .instance
            .get_physical_device_memory_properties(renderer.physical_device)
    };

    let mem_type_index = mem_props
        .memory_types
        .iter()
        .enumerate()
        .find(|(i, ty)| {
            (mem_reqs.memory_type_bits & (1 << i)) != 0
                && ty
                    .property_flags
                    .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
        })
        .map(|(i, _)| i as u32)
        .expect("Failed to find suitable memory type");

    let mut import_info = vk::ImportMemoryFdInfoKHR::default()
        .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
        .fd(raw_fd);

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_reqs.size)
        .memory_type_index(mem_type_index)
        .push_next(&mut import_info);

    let vk_memory = unsafe { renderer.device.allocate_memory(&alloc_info, None)? };
    unsafe { renderer.device.bind_image_memory(vk_image, vk_memory, 0)? };

    // Store complete state on the card so the timer loop can access it
    card.surface = Some(CardSurface {
        gbm_bo,
        vk_image,
        vk_memory,
        fb,
        crtc,
    });

    tracing::info!("Vulkan surface initialized and modeset complete!");
    Ok(())
}

fn draw_color(
    renderer: &crate::renderer::Renderer,
    surface: &CardSurface,
    color: [f32; 4],
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(renderer.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = renderer.device.allocate_command_buffers(&alloc_info)?[0];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        renderer.device.begin_command_buffer(cmd, &begin_info)?;

        // Transition to GENERAL layout for LINEAR images
        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL) // Changed from TRANSFER_DST_OPTIMAL
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(surface.vk_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

        renderer.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );

        let clear_color = vk::ClearColorValue { float32: color };
        let range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        // Execute clear using GENERAL layout
        renderer.device.cmd_clear_color_image(
            cmd,
            surface.vk_image,
            vk::ImageLayout::GENERAL, // Changed from TRANSFER_DST_OPTIMAL
            &clear_color,
            &[range],
        );

        // Final barrier to ensure memory is flushed for DRM scanout
        let present_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(surface.vk_image)
            .subresource_range(range)
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::MEMORY_READ);

        renderer.device.cmd_pipeline_barrier(
            cmd,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[present_barrier],
        );

        renderer.device.end_command_buffer(cmd)?;

        let submit_info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
        renderer
            .device
            .queue_submit(renderer.graphics_queue, &[submit_info], vk::Fence::null())?;

        renderer.device.queue_wait_idle(renderer.graphics_queue)?;
        renderer
            .device
            .free_command_buffers(renderer.command_pool, &[cmd]);
    }

    Ok(())
}
