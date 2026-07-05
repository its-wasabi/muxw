use input::event::keyboard::KeyboardEventTrait;
use std::os::fd::AsRawFd;

pub struct WorkerResources {
    poller: polling::Poller,
    monitor: udev::MonitorSocket,
    libinput: input::Libinput,

    active_devices: std::collections::HashMap<std::path::PathBuf, input::Device>,
}

impl WorkerResources {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut libinput = input::Libinput::new_from_path(LibinputInterface {});
        let mut active_devices: std::collections::HashMap<std::path::PathBuf, input::Device> =
            std::collections::HashMap::new();

        let mut enumerator = udev::Enumerator::new()?;
        enumerator.match_subsystem("input")?;

        for udev_device in enumerator.scan_devices()? {
            if let std::option::Option::Some(devnode) = udev_device.devnode()
                && let std::option::Option::Some(path_str) = devnode.to_str()
                && let std::option::Option::Some(device) = libinput.path_add_device(path_str)
            {
                active_devices.insert(devnode.to_path_buf(), device);
            }
        }

        let monitor = udev::MonitorBuilder::new()?
            .match_subsystem("input")?
            .listen()?;

        let poller = polling::Poller::new()?;
        let libinput_fd = libinput.as_raw_fd();
        let monitor_fd = monitor.as_raw_fd();

        unsafe {
            poller.add_with_mode(
                libinput_fd,
                polling::Event::readable(1),
                polling::PollMode::Edge,
            )?;
            poller.add_with_mode(
                monitor_fd,
                polling::Event::readable(2),
                polling::PollMode::Edge,
            )?;
        }

        Ok(Self {
            poller,
            monitor,
            libinput,

            active_devices,
        })
    }
}

// SAFETY: WorkerResources is fully constructed on the calling thread and then
// moved in its entirety into the worker thread's closure. After the move, no
// other thread ever touches libinput/monitor/poller/active_devices, and none
// of them are accessed concurrently from two threads. Libinput/udev's C
// objects have no documented same-thread-only requirement, they just aren't
// safe to use from multiple threads *at once*, which this usage never does.
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for WorkerResources {}

#[allow(clippy::needless_pass_by_value)]
pub fn run_input_worker_thread(
    mut resources: WorkerResources,
    sender: crate::event_loop::EventSender<crate::Token>,
) -> ! {
    let mut events = polling::Events::new();
    drain_libinput_events(&mut resources.libinput, &sender);

    loop {
        events.clear();
        resources.poller.wait(&mut events, None); // TODO: <- Errors

        for event in events.iter() {
            if event.key == 1 {
                drain_libinput_events(&mut resources.libinput, &sender);
            } else if event.key == 2 {
                for udev_event in resources.monitor.iter() {
                    if let std::option::Option::Some(devnode) = udev_event.devnode() {
                        match udev_event.event_type() {
                            udev::EventType::Add => {
                                if let std::option::Option::Some(path_str) = devnode.to_str() {
                                    if let std::option::Option::Some(device) =
                                        resources.libinput.path_add_device(path_str)
                                    {
                                        resources
                                            .active_devices
                                            .insert(devnode.to_path_buf(), device);
                                    }
                                }
                            }
                            udev::EventType::Remove => {
                                if let std::option::Option::Some(device) =
                                    resources.active_devices.remove(devnode)
                                {
                                    resources.libinput.path_remove_device(device);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn drain_libinput_events(
    libinput: &mut input::Libinput,
    sender: &crate::event_loop::EventSender<crate::Token>,
) {
    libinput.dispatch(); // <- TODO: Errors
    for event in libinput {
        match event {
            input::Event::Device(_) => {
                let input_event = super::InputEvent::Device {};
                sender.send(crate::Token::Input(input_event));
            }
            input::Event::Keyboard(keyboard_event) => {
                let input_event = super::InputEvent::Keyboard {
                    keycode: keyboard_event.key(),
                    state: keyboard_event.key_state(),
                };
                sender.send(crate::Token::Input(input_event));
            }
            input::Event::Pointer(pointer_event) => {
                if let input::event::pointer::PointerEvent::Motion(motion) = pointer_event {
                    let input_event = super::InputEvent::Pointer {
                        delta_x: motion.dx(),
                        delta_y: motion.dy(),
                    };
                    sender.send(crate::Token::Input(input_event));
                }
            }
            // input::Event::Touch(touch_event) => todo!(),
            // input::Event::Tablet(tablet_tool_event) => todo!(),
            // input::Event::TabletPad(tablet_pad_event) => todo!(),
            // input::Event::Gesture(gesture_event) => todo!(),
            // input::Event::Switch(switch_event) => todo!(),
            _ => {}
        }
    }
}

pub struct LibinputInterface {}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        use std::os::unix::fs::OpenOptionsExt;
        std::println!(
            "\x1b[38;5;87mInput::ADD::(>\x1b[38;5;198m {} \x1b[38;5;87m<)\x1b[0m",
            path.display()
        );

        let access_mode = flags & libc::O_ACCMODE;
        let is_read = access_mode == libc::O_RDONLY || access_mode == libc::O_RDWR;
        let is_write = access_mode == libc::O_WRONLY || access_mode == libc::O_RDWR;

        std::fs::OpenOptions::new()
            .custom_flags(flags)
            .read(is_read)
            .write(is_write)
            .open(path)
            .map(std::convert::Into::into)
            .map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        std::println!(
            "\x1b[38;5;87mInput::REMOTE::(>\x1b[38;5;198m {:?} \x1b[38;5;87m<)\x1b[0m",
            fd
        );
        std::mem::drop(std::fs::File::from(fd));
    }
}
