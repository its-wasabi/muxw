use std::os::fd::AsRawFd;

pub struct InputManager {
    libinput: input::Libinput,
    xkb_state: xkbcommon::xkb::State,
    keymap: xkbcommon::xkb::Keymap,
}

impl InputManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut libinput = input::Libinput::new_with_udev(LibinputInterface::new());
        libinput.udev_assign_seat("seat0").expect("Whoops");

        let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let keymap = xkbcommon::xkb::Keymap::new_from_names(
            &xkb_context,
            "",
            "",
            "",
            "",
            None,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or("failed to create xkb keymap")?;
        let xkb_state = xkbcommon::xkb::State::new(&keymap);

        Ok(Self {
            libinput,
            xkb_state,
            keymap,
        })
    }

    pub fn dispatch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.libinput.dispatch()?;
        for event in &mut self.libinput {
            if let input::Event::Keyboard(kb) = event {
                println!("INPUT-MANAGER: {kb:#?}");
            }
        }

        Ok(())
    }

    pub fn fd(&self) -> std::os::fd::BorrowedFd {
        let raw_fd = self.libinput.as_raw_fd();
        unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) }
    }
}

struct LibinputInterface {}

impl LibinputInterface {
    const fn new() -> Self {
        Self {}
    }
}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        flags: i32,
    ) -> Result<std::os::fd::OwnedFd, i32> {
        println!("INPUT-DEV: {}", path.display());
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .custom_flags(flags)
            .read((flags & libc::O_RDONLY != 0) | (flags & libc::O_RDWR != 0))
            .write((flags & libc::O_WRONLY != 0) | (flags & libc::O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        unsafe {
            std::fs::File::from(fd);
        }
    }
}
