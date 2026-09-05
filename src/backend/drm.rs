// TODO: Handle card dynamically
// 2. Best card selection (not best but make use of all cards that have monitor connected)
// 3. Config API for card selection

// IMPORTANT: For reference and valid usage read your older code on gitea cause this shit is broken
// vibecoded scum and should not be used for other purpose than high level overview

// IMPORTANT: Hold event_loop key to deregister event source when removing device
// IMPORTANT: Merge monitor and enumerator device registration logic (mostly is_primary check)
// IMPORTANT: Also error handling here is non-existent if device cant be opened instead of
// compositor crash it should be simply ignored

// TODO: Try to make DrmCard have Drop trait implemented that will automatically deregister that
// from event loop.

use ash::vk;
use drm::Device as _;
use drm::control::Device as _;
use drm::control::atomic::AtomicModeReq;
use drm::control::{AtomicCommitFlags, property};
use std::collections::HashMap;
use std::os::fd::{AsFd, BorrowedFd};
use std::path::Path;

const DEVICES_DEFAULT_CAPACITY: usize = 4;

pub struct DrmManager {
    cards: slab::Slab<DrmCard>,
    monitor: udev::MonitorSocket,
    device_keys: HashMap<u64, DrmCardKey>,
    active: bool,
}

impl DrmManager {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop,
        renderer: &crate::renderer::Renderer,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!("Initializing DRM subsystem with Atomic KMS...");

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
            device_keys: HashMap::with_capacity(DEVICES_DEFAULT_CAPACITY),
            active: true,
        };

        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("drm")?;

        for drm_device in enumerator.scan_devices()? {
            drm_manager.process_udev_add(drm_device, event_loop, renderer);
        }

        Ok(drm_manager)
    }

    pub fn drop_master(&mut self) {
        for (_, card) in self.cards.iter_mut() {
            let _ = card.release_master_lock();
        }
    }

    pub fn pause(&mut self) {
        self.active = false;
        tracing::info!("DRM subsystem paused via TTY switch");
    }

    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.active = true;
        tracing::info!("DRM subsystem resumed, restoring display modesets...");

        for (_, card) in self.cards.iter_mut() {
            for surface in &mut card.surfaces {
                surface.flip_pending = false;
                surface.needs_repaint = true;
            }

            for i in 0..card.surfaces.len() {
                let (crtc, front_fb, connector, mode) = {
                    let surface = &card.surfaces[i];
                    let front_fb = surface.buffers[1 - surface.current_back].fb;
                    (surface.crtc, front_fb, surface.connector, surface.mode)
                };

                if let Err(e) =
                    card.set_crtc(crtc, Some(front_fb), (0, 0), &[connector], Some(mode))
                {
                    tracing::error!("Failed to restore CRTC {:?}: {}", crtc, e);
                }
            }
        }
        Ok(())
    }

    pub fn dispatch_udev(
        &mut self,
        event_loop: &mut crate::event_loop::EventLoop,
        renderer: &crate::renderer::Renderer,
    ) {
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

    pub fn dispatch_card(
        &mut self,
        key: DrmCardKey,
        renderer: &crate::renderer::Renderer,
        color: [f32; 4],
    ) {
        if !self.active {
            return;
        }

        let Some(card) = self.cards.get_mut(key.get()) else {
            return;
        };

        match card.receive_events() {
            Ok(events) => {
                for event in events {
                    if let drm::control::Event::PageFlip(flip) = event {
                        for surface_idx in 0..card.surfaces.len() {
                            let surface = &mut card.surfaces[surface_idx];
                            if surface.crtc == flip.crtc {
                                surface.flip_pending = false;
                                surface.current_back = 1 - surface.current_back;

                                if surface.needs_repaint {
                                    if let Err(e) = draw_color(renderer, card, surface_idx, color) {
                                        tracing::error!(
                                            "Failed deferred redraw on surface {}: {}",
                                            surface_idx,
                                            e
                                        );
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => tracing::error!("Error reading DRM events on card {}: {}", key.get(), e),
        }
    }

    pub fn paint_all(&mut self, renderer: &crate::renderer::Renderer, color: [f32; 4]) {
        if !self.active {
            return;
        }

        for (_, card) in self.cards.iter_mut() {
            for surface_idx in 0..card.surfaces.len() {
                if let Err(e) = draw_color(renderer, card, surface_idx, color) {
                    tracing::error!("Failed to draw frame on surface {}: {}", surface_idx, e);
                }
            }
        }
    }

    fn process_udev_add(
        &mut self,
        device: udev::Device,
        event_loop: &mut crate::event_loop::EventLoop,
        renderer: &crate::renderer::Renderer,
    ) {
        let sysname = device.sysname().to_string_lossy();
        if !sysname.starts_with("card") || sysname.contains('-') {
            return;
        }

        let (Some(path), Some(devnum)) = (device.devnode(), device.devnum()) else {
            return;
        };

        if rustix::fs::minor(devnum) > 63 {
            return;
        }

        if self.device_keys.contains_key(&devnum) {
            return;
        }

        match self.register_card_inner(path, devnum, event_loop) {
            Ok(key) => {
                tracing::info!("Registered DRM card: {:?}", path);
                self.device_keys.insert(devnum, key);

                if let Some(card) = self.cards.get_mut(key.get()) {
                    if let Err(e) = init_surface(renderer, card) {
                        tracing::warn!(
                            "Failed to initialize display surfaces for {:?}: {}",
                            path,
                            e
                        );
                    }
                }
            }
            Err(e) => tracing::error!("Failed to open/register DRM card {:?}: {}", path, e),
        }
    }

    fn register_card_inner(
        &mut self,
        path: &Path,
        devnum: u64,
        event_loop: &mut crate::event_loop::EventLoop,
    ) -> Result<DrmCardKey, Box<dyn std::error::Error>> {
        let vacant_entry = self.cards.vacant_entry();
        let key = DrmCardKey(vacant_entry.key());

        let mut card = DrmCard::new(path, devnum)?;

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
        event_loop: &mut crate::event_loop::EventLoop,
    ) {
        if self.cards.contains(key.get()) {
            let card = self.cards.remove(key.get());
            if let Some(io_key) = card.event_loop_key {
                event_loop.deregister_source(io_key, &card);
            }
            tracing::info!("Deregistered DRM card {}", key.get());
        }
    }
}

pub struct SurfaceBuffer {
    pub gbm_bo: gbm::BufferObject<()>,
    pub vk_image: ash::vk::Image,
    pub vk_memory: ash::vk::DeviceMemory,
    pub fb: drm::control::framebuffer::Handle,
}

pub struct CardSurface {
    pub buffers: [SurfaceBuffer; 2],
    pub current_back: usize,

    pub crtc: drm::control::crtc::Handle,
    pub connector: drm::control::connector::Handle,
    pub mode: drm::control::Mode,

    pub plane: drm::control::plane::Handle,
    pub plane_fb_prop: property::Handle,
    pub plane_crtc_prop: property::Handle,

    pub flip_pending: bool,
    pub needs_repaint: bool,
}

pub struct DrmCard {
    file: std::fs::File,
    pub devnum: u64,
    pub event_loop_key: Option<crate::event_loop::IoKey>,
    pub surfaces: Vec<CardSurface>,
}

impl DrmCard {
    fn new(path: &Path, devnum: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        let card = Self {
            file,
            devnum,
            event_loop_key: None,
            surfaces: Vec::new(),
        };

        card.set_atomic_universal_planes(true)?;
        Ok(card)
    }

    fn set_atomic_universal_planes(&self, enable: bool) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.set_client_capability(drm::ClientCapability::UniversalPlanes, enable);
        let _ = self.set_client_capability(drm::ClientCapability::Atomic, enable);
        Ok(())
    }
}

impl Drop for DrmCard {
    fn drop(&mut self) {
        for surface in &self.surfaces {
            let _ = self.destroy_framebuffer(surface.buffers[0].fb);
            let _ = self.destroy_framebuffer(surface.buffers[1].fb);
        }
    }
}

impl AsFd for DrmCard {
    fn as_fd(&self) -> BorrowedFd<'_> {
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
    card: &mut DrmCard,
) -> Result<(), Box<dyn std::error::Error>> {
    let res_handles = card.resource_handles()?;
    let crtcs = res_handles.crtcs();

    let mut assigned_crtc_count = 0;
    let mut active_surfaces = Vec::new();

    let connected_connectors: Vec<_> = res_handles
        .connectors()
        .iter()
        .filter_map(|&conn| card.get_connector(conn, true).ok())
        .filter(|c| c.state() == drm::control::connector::State::Connected)
        .collect();

    if connected_connectors.is_empty() {
        tracing::info!("No connected monitors on DRM devnum {}", card.devnum);
        return Ok(());
    }

    let plane_handles = card.plane_handles()?;
    let gbm = gbm::Device::new(&*card).map_err(|e| format!("GBM init failed: {}", e))?;

    for connector in connected_connectors {
        let modes = connector.modes();
        if modes.is_empty() {
            continue;
        }
        let mode = modes[0];

        if assigned_crtc_count >= crtcs.len() {
            tracing::warn!(
                "Connected monitors exceed available CRTCs on devnum {}",
                card.devnum
            );
            break;
        }
        let crtc = crtcs[assigned_crtc_count];
        assigned_crtc_count += 1;

        // Find primary plane compatible with CRTC using res_handles.filter_crtcs(...)
        let mut plane_info = None;
        for &p in &plane_handles {
            if let Ok(p_info) = card.get_plane(p) {
                if res_handles
                    .filter_crtcs(p_info.possible_crtcs())
                    .contains(&crtc)
                {
                    let props = card.get_properties(p)?;
                    let mut fb_prop = None;
                    let mut crtc_prop = None;

                    for (prop_handle, _) in props {
                        if let Ok(prop_info) = card.get_property(prop_handle) {
                            match prop_info.name().to_str() {
                                Ok("FB_ID") => fb_prop = Some(prop_handle),
                                Ok("CRTC_ID") => crtc_prop = Some(prop_handle),
                                _ => {}
                            }
                        }
                    }

                    if let (Some(fb_p), Some(crtc_p)) = (fb_prop, crtc_prop) {
                        plane_info = Some((p, fb_p, crtc_p));
                        break;
                    }
                }
            }
        }

        let (plane, plane_fb_prop, plane_crtc_prop) = plane_info
            .ok_or_else(|| format!("No compatible atomic plane found for CRTC {:?}", crtc))?;

        let (width, height) = mode.size();

        let create_surface_buffer = || -> Result<SurfaceBuffer, Box<dyn std::error::Error>> {
            let gbm_bo = gbm.create_buffer_object::<()>(
                width as u32,
                height as u32,
                gbm::Format::Xrgb8888,
                gbm::BufferObjectFlags::SCANOUT
                    | gbm::BufferObjectFlags::RENDERING
                    | gbm::BufferObjectFlags::LINEAR,
            )?;

            use std::os::fd::IntoRawFd;
            let dma_buf_fd = gbm_bo
                .fd()
                .map_err(|e| format!("Failed to export DMA fd: {:?}", e))?;
            let raw_fd = dma_buf_fd.into_raw_fd();

            let raw_handle = unsafe { gbm_bo.handle().u32_ };
            let non_zero_handle =
                std::num::NonZeroU32::new(raw_handle).ok_or("GBM returned zero handle")?;

            let bridge_buffer = GbmToDrm {
                size: (width as u32, height as u32),
                pitch: gbm_bo.stride(),
                handle: drm::buffer::Handle::from(non_zero_handle),
                format: drm::buffer::DrmFourcc::Xrgb8888,
            };

            let fb = card.add_framebuffer(&bridge_buffer, 24, 32)?;

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
                .ok_or("Failed to find suitable Vulkan memory type")?;

            let mut import_info = vk::ImportMemoryFdInfoKHR::default()
                .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
                .fd(raw_fd);

            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_reqs.size)
                .memory_type_index(mem_type_index)
                .push_next(&mut import_info);

            let vk_memory = unsafe { renderer.device.allocate_memory(&alloc_info, None)? };
            unsafe { renderer.device.bind_image_memory(vk_image, vk_memory, 0)? };

            Ok(SurfaceBuffer {
                gbm_bo,
                vk_image,
                vk_memory,
                fb,
            })
        };

        let buf0 = create_surface_buffer()?;
        let buf1 = create_surface_buffer()?;

        card.set_crtc(
            crtc,
            Some(buf0.fb),
            (0, 0),
            &[connector.handle()],
            Some(mode),
        )?;

        active_surfaces.push(CardSurface {
            buffers: [buf0, buf1],
            current_back: 1,
            crtc,
            connector: connector.handle(),
            mode,
            plane,
            plane_fb_prop,
            plane_crtc_prop,
            flip_pending: false,
            needs_repaint: false,
        });
    }

    card.surfaces = active_surfaces;
    Ok(())
}

fn draw_color(
    renderer: &crate::renderer::Renderer,
    card: &mut DrmCard,
    surface_idx: usize,
    color: [f32; 4],
) -> Result<(), Box<dyn std::error::Error>> {
    if card.surfaces[surface_idx].flip_pending {
        card.surfaces[surface_idx].needs_repaint = true;
        return Ok(());
    }

    let back_idx = card.surfaces[surface_idx].current_back;
    let vk_image = card.surfaces[surface_idx].buffers[back_idx].vk_image;
    let fb = card.surfaces[surface_idx].buffers[back_idx].fb;
    let crtc = card.surfaces[surface_idx].crtc;
    let plane = card.surfaces[surface_idx].plane;
    let plane_fb_prop = card.surfaces[surface_idx].plane_fb_prop;
    let plane_crtc_prop = card.surfaces[surface_idx].plane_crtc_prop;

    unsafe {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(renderer.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let cmd = renderer.device.allocate_command_buffers(&alloc_info)?[0];

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        renderer.device.begin_command_buffer(cmd, &begin_info)?;

        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(vk_image)
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

        renderer.device.cmd_clear_color_image(
            cmd,
            vk_image,
            vk::ImageLayout::GENERAL,
            &clear_color,
            &[range],
        );

        let present_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(vk_image)
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

    // DRM Atomic Commit execution
    let mut atomic_req = AtomicModeReq::new();
    atomic_req.add_property(
        plane,
        plane_fb_prop,
        drm::control::property::Value::Framebuffer(Some(fb)),
    );
    atomic_req.add_property(
        plane,
        plane_crtc_prop,
        drm::control::property::Value::CRTC(Some(crtc)),
    );

    let flags = AtomicCommitFlags::PAGE_FLIP_EVENT | AtomicCommitFlags::NONBLOCK;

    match card.atomic_commit(flags, atomic_req) {
        Ok(()) => {
            let surface = &mut card.surfaces[surface_idx];
            surface.flip_pending = true;
            surface.needs_repaint = false;
        }
        Err(e) => {
            tracing::error!("Atomic commit failed on CRTC {:?}: {}", crtc, e);
            card.surfaces[surface_idx].needs_repaint = true;
        }
    }

    Ok(())
}
