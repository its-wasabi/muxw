use input::event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent};
use std::os::fd::AsFd;

pub struct InputWorker {
    event_loop: crate::event_loop::EventLoop<Token>,
    libinput: input::Libinput,
    sender: crate::event_loop::EventSender<crate::Token>,

    devices: slab::Slab<input::Device>,
    device_keys: std::collections::HashMap<input::Device, super::InputDeviceKey>,
}

impl InputWorker {
    pub fn new(
        sender: crate::event_loop::EventSender<crate::Token>,
    ) -> Result<(crate::event_loop::EventSender<Token>, Self), Box<dyn std::error::Error>> {
        let mut event_loop = crate::event_loop::EventLoop::new()?;

        let mut libinput = input::Libinput::new_with_udev(LibinputInterface {});
        // TODO: Handle error
        libinput.udev_assign_seat("seat0");
        event_loop.register_source(&libinput.as_fd(), polling::PollMode::Edge, Token::Libinput)?;

        Ok((
            event_loop.channel_sender(),
            Self {
                event_loop,
                libinput,
                sender,
                devices: slab::Slab::with_capacity(1),
                device_keys: std::collections::HashMap::with_capacity(1),
            },
        ))
    }
}

// SAFETY: InputWorker is fully constructed on the calling thread and then moved
// in its entirety into the worker thread's closure. After the move, no other
// thread ever touches it, and none of them are
// accessed concurrently from two threads. Libinput's C objects have no
// documented same-thread-only requirement, they just aren't safe to use from
// multiple threads at once, which this usage never does.
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for InputWorker {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    Libinput,
}

pub struct LibinputInterface {}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        let oflags = rustix::fs::OFlags::from_bits_truncate(flags.cast_unsigned());
        rustix::fs::open(path, oflags, rustix::fs::Mode::empty())
            .map_err(rustix::io::Errno::raw_os_error)
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        std::mem::drop(std::fs::File::from(fd));
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn run_input_worker_thread(mut worker: InputWorker) -> ! {
    let mut triggered = Vec::with_capacity(4);
    drain_libinput_events(&mut worker);

    loop {
        if let Err(error) = worker.event_loop.dispatch(&mut triggered) {
            log::error!("Input worker failed to dispatch EventLoop: {error:?}");
            continue;
        }

        for token in &triggered {
            match token {
                Token::Libinput => drain_libinput_events(&mut worker),
            }
        }
    }
}

fn drain_libinput_events(worker: &mut InputWorker) {
    let InputWorker {
        libinput,
        devices,
        device_keys,
        sender,
        ..
    } = worker;

    if let Err(error) = libinput.dispatch() {
        log::error!("Input worker failed to dispatch Libinput: {error:?}");
        return;
    }

    for event in &mut *libinput {
        match event {
            input::Event::Device(input::event::DeviceEvent::Added(device_add_event)) => {
                let device = device_add_event.device();
                let key = super::InputDeviceKey(devices.insert(device.clone()));
                device_keys.insert(device, key);
                sender.send(crate::Token::Input(super::InputEvent {
                    key,
                    kind: super::InputEventKind::DeviceAdded,
                }));
            }

            input::Event::Device(input::event::DeviceEvent::Removed(device_remove_event)) => {
                let device = device_remove_event.device();
                if let Some(key) = device_keys.remove(&device) {
                    devices.remove(key.get());
                    sender.send(crate::Token::Input(super::InputEvent {
                        key,
                        kind: super::InputEventKind::DeviceRemoved,
                    }));
                }
            }

            input::Event::Keyboard(keyboard_event) => {
                let device = keyboard_event.device();
                if let Some(key) = device_keys.get(&device) {
                    sender.send(crate::Token::Input(super::InputEvent {
                        key: *key,
                        kind: super::InputEventKind::Keyboard {
                            keycode: keyboard_event.key(),
                            state: keyboard_event.key_state(),
                        },
                    }));
                }
            }

            input::Event::Pointer(input::event::PointerEvent::Button(pointer_button_event)) => {
                let device = pointer_button_event.device();
                if let Some(key) = device_keys.get(&device) {
                    sender.send(crate::Token::Input(super::InputEvent {
                        key: *key,
                        kind: super::InputEventKind::PointerButton {
                            keycode: pointer_button_event.button(),
                            state: pointer_button_event.button_state(),
                        },
                    }));
                }
            }

            input::Event::Pointer(input::event::PointerEvent::ScrollWheel(
                pointer_scroll_wheel_event,
            )) => {
                let device = pointer_scroll_wheel_event.device();
                if let Some(key) = device_keys.get(&device) {
                    if pointer_scroll_wheel_event.has_axis(input::event::pointer::Axis::Vertical) {
                        let vertical = pointer_scroll_wheel_event
                            .scroll_value(input::event::pointer::Axis::Vertical);

                        if vertical != 0.0 {
                            sender.send(crate::Token::Input(super::InputEvent {
                                key: *key,
                                kind: super::InputEventKind::PointerVerticalScroll {
                                    scroll: vertical,
                                },
                            }));
                        }
                    }

                    if pointer_scroll_wheel_event.has_axis(input::event::pointer::Axis::Horizontal)
                    {
                        let horizontal = pointer_scroll_wheel_event
                            .scroll_value(input::event::pointer::Axis::Horizontal);

                        if horizontal != 0.0 {
                            sender.send(crate::Token::Input(super::InputEvent {
                                key: *key,
                                kind: super::InputEventKind::PointerHorizontalScroll {
                                    scroll: horizontal,
                                },
                            }));
                        }
                    }
                }
            }

            input::Event::Pointer(input::event::PointerEvent::Motion(motion)) => {
                let device = motion.device();
                if let Some(key) = device_keys.get(&device) {
                    sender.send(crate::Token::Input(super::InputEvent {
                        key: *key,
                        kind: super::InputEventKind::Motion {
                            delta_x: motion.dx(),
                            delta_y: motion.dy(),
                        },
                    }));
                }
            }
            _ => {}
        }
    }
}
