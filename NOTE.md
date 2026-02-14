# Optimizations
- [ ] Implement damage tracking
- [ ] Buffer age (skip redraw if nothing changed)
- [ ] Pass buffer directly to KMS if full screen or conditions allow


# NOTES
## Config
### Mlua
- [ ] Precompile lua functions to byte code by calling .eval() on them before actually runngin them
- [ ] Parse as much of config as possible into rust native structures during load
### State
- [?] maybe instead of creating snapshots of config use some atomic lock approach
   - It could create problem in which each process locks the resource and the other need to wait for
     it which isnt ideal even worse at startup
## Graphics
### Vulkan creation with wayland
1. for `WaylandSurfaceCreateInfo` you need wayland display so create that before creating vulkan
   context and pass it to the vulkan.
2. While creating `Swapchain` use wayland_surface created from `WaylandSurfaceCreateInfo` as surface
   of the swapchain.
### Handling external memory
```c
VkExternalMemoryImageCreateInfo external_info = {
    .sType = VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
    .handleTypes = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
};

VkImageCreateInfo image_info = {
    .sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
    .pNext = &external_info,
    .imageType = VK_IMAGE_TYPE_2D,
    .format = VK_FORMAT_B8G8R8A8_UNORM,
    .extent = {width, height, 1},
    .usage = VK_IMAGE_USAGE_SAMPLED_BIT,
    // ...
};
vkCreateImage(device, &image_info, NULL, &client_image);

// Import the DMA-BUF fd
VkImportMemoryFdInfoKHR import_info = {
    .sType = VK_STRUCTURE_TYPE_IMPORT_MEMORY_FD_INFO_KHR,
    .handleType = VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT,
    .fd = dmabuf_fd,  // From client
};

VkMemoryAllocateInfo alloc_info = {
    .sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
    .pNext = &import_info,
    .allocationSize = memory_requirements.size,
    .memoryTypeIndex = memory_type_index,
};
vkAllocateMemory(device, &alloc_info, NULL, &memory);
vkBindImageMemory(device, client_image, memory, 0);
```
As you can see you have to:
1. create external_info pass it as an next to image_info.
2. Create image.
3. create import_info
4. create alloc_info which takes import_info as next
5. allocate memory for the image TODO:(Isn't that expensive check if there's more efficient way).
6. bind memory with client image and allocated memory.
### GBM handling
You don't need to use GBM in vulkan compositor cause it probably arrives as DMA-BUF anyway.
