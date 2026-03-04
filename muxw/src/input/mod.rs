// use input::event::keyboard::{KeyState, KeyboardEventTrait};
// use std::os::unix::io::AsRawFd;
//
// pub struct InputManager {
//     libinput: input::Libinput,
//     xkb_state: xkbcommon::xkb::State,
//     keymap: xkbcommon::xkb::Keymap, // must outlive xkb_state
// }
//
// #[derive(Debug)]
// pub struct Key {
//     pub keycode: u32,
//     pub keysym: xkbcommon::xkb::Keysym,
//     pub utf8: Option<String>,
//     pub state: KeyState,
//     pub time_usec: u64,
// }
//
// impl InputManager {
//     pub fn new(epoll_fd: std::os::unix::io::RawFd) -> Result<Self, Box<dyn std::error::Error>> {
//         let mut libinput = input::Libinput::new_with_udev(LibinputInterface::new());
//         libinput.udev_assign_seat("seat0").expect("Whoops");
//
//         muxw_epoll::add_fd(epoll_fd, libinput.as_raw_fd(), muxw_epoll::Interest::READ)?;
//
//         let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
//         let keymap = xkbcommon::xkb::Keymap::new_from_names(
//             &xkb_context,
//             "",
//             "",
//             "",
//             "",
//             None,
//             xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
//         )
//         .ok_or("failed to create xkb keymap")?;
//         let xkb_state = xkbcommon::xkb::State::new(&keymap);
//
//         Ok(Self {
//             libinput,
//             xkb_state,
//             keymap,
//         })
//     }
//
//     /// Call this when epoll fires on the libinput fd
//     pub fn dispatch(&mut self, cb: impl FnMut(Key)) {
//         self.libinput.dispatch().unwrap();
//         let mut cb = cb;
//         for event in &mut self.libinput {
//             if let input::Event::Keyboard(kb) = event {
//                 let keycode = kb.key() + 8; // evdev → xkb offset
//                 let direction = match kb.key_state() {
//                     KeyState::Pressed => xkbcommon::xkb::KeyDirection::Down,
//                     KeyState::Released => xkbcommon::xkb::KeyDirection::Up,
//                 };
//                 self.xkb_state.update_key(keycode, direction);
//
//                 let keysym = self.xkb_state.key_get_one_sym(keycode);
//                 let utf8 = {
//                     let s = self.xkb_state.key_get_utf8(keycode);
//                     if s.is_empty() { None } else { Some(s) }
//                 };
//
//                 cb(Key {
//                     keycode,
//                     keysym,
//                     utf8,
//                     state: kb.key_state(),
//                     time_usec: kb.time_usec(),
//                 });
//             }
//         }
//     }
//
//     pub fn fd(&self) -> std::os::unix::io::RawFd {
//         self.libinput.as_raw_fd()
//     }
// }
//
// struct LibinputInterface {}
//
// impl LibinputInterface {
//     const fn new() -> Self {
//         Self {}
//     }
// }
//
// impl input::LibinputInterface for LibinputInterface {
//     fn open_restricted(
//         &mut self,
//         path: &std::path::Path,
//         flags: i32,
//     ) -> Result<std::os::fd::OwnedFd, i32> {
//         use std::os::unix::fs::OpenOptionsExt;
//         std::fs::OpenOptions::new()
//             .custom_flags(flags)
//             .read((flags & libc::O_RDONLY != 0) | (flags & libc::O_RDWR != 0))
//             .write((flags & libc::O_WRONLY != 0) | (flags & libc::O_RDWR != 0))
//             .open(path)
//             .map(|file| file.into())
//             .map_err(|err| err.raw_os_error().unwrap())
//     }
//     fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
//         unsafe {
//             std::fs::File::from(fd);
//         }
//     }
// }
