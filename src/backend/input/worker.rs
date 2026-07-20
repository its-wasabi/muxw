use input::event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent};
use std::os::fd::{AsFd, AsRawFd};

pub struct InputWorker {
    event_loop: crate::event_loop::EventLoop<InputToken>,
    libinput: input::Libinput,
    sender: crate::event_loop::EventSender<crate::token::Token>,

    devices: slab::Slab<input::Device>,
    device_keys: std::collections::HashMap<input::Device, super::InputDeviceKey>,
}

impl InputWorker {
    pub fn new(
        sender: crate::event_loop::EventSender<crate::token::Token>,
    ) -> Result<(crate::event_loop::EventSender<InputToken>, Self), Box<dyn std::error::Error>>
    {
        let mut inner_event_loop = crate::event_loop::EventLoop::new()?;
        let libinput = input::Libinput::new_with_udev(LibinputInterface {
            sender: sender.clone(),
        });

        inner_event_loop.register_source(
            &libinput.as_fd(),
            polling::PollMode::Edge,
            InputToken::Libinput,
        )?;

        Ok((
            inner_event_loop.sender(),
            Self {
                event_loop: inner_event_loop,
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
pub(super) enum InputToken {
    Libinput,
}

pub struct LibinputInterface {
    sender: crate::event_loop::EventSender<crate::token::Token>,
}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        _flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        let (reply, response) = crossbeam_channel::bounded(1);

        self.sender
            .send(crate::token::Token::SeatOpenRequest(Box::new(
                crate::token::SeatOpenData {
                    path: path.to_path_buf(),
                    reply,
                },
            )));

        match response.recv() {
            Ok(Ok(fd)) => Ok(fd),
            Ok(Err(io_error)) => Err(io_error.raw_os_error().unwrap_or(13)),
            Err(_) => Err(13),
        }
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        let raw_fd = fd.as_raw_fd();
        std::mem::drop(fd);
        self.sender
            .send(crate::token::Token::SeatCloseRequest(raw_fd));
    }
}

#[allow(clippy::needless_pass_by_value)]
pub fn run_input_worker_thread(mut worker: InputWorker) -> ! {
    let seat = "seat0";

    let _span_guard = tracing::error_span!("input", ?seat).entered();

    if worker.libinput.udev_assign_seat(seat) == Err(()) {
        todo!(
            "Move seat initialization into InputWorger::new() or make it fully dynamic not and configurable"
        );
    }

    let mut triggered = Vec::with_capacity(4);
    drain_libinput_events(&mut worker);

    loop {
        if let Err(error) = worker.event_loop.dispatch(&mut triggered) {
            tracing::error!(?error, "Input worker failed to dispatch EventLoop");
            continue;
        }

        for token in &triggered {
            match token {
                InputToken::Libinput => drain_libinput_events(&mut worker),
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
        tracing::error!(?error, "Input worker failed to dispatch libinput");
        return;
    }

    for event in &mut *libinput {
        match event {
            input::Event::Device(input::event::DeviceEvent::Added(device_add_event)) => {
                let device = device_add_event.device();
                let key = super::InputDeviceKey(devices.insert(device.clone()));
                device_keys.insert(device, key);
                sender.send(crate::token::Token::Input(super::InputEvent {
                    key,
                    kind: super::InputEventKind::DeviceAdded,
                }));
            }

            input::Event::Device(input::event::DeviceEvent::Removed(device_remove_event)) => {
                let device = device_remove_event.device();
                if let Some(key) = device_keys.remove(&device) {
                    devices.remove(key.get());
                    sender.send(crate::token::Token::Input(super::InputEvent {
                        key,
                        kind: super::InputEventKind::DeviceRemoved,
                    }));
                }
            }

            input::Event::Keyboard(keyboard_event) => {
                let device = keyboard_event.device();
                if let Some(key) = device_keys.get(&device) {
                    sender.send(crate::token::Token::Input(super::InputEvent {
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
                    sender.send(crate::token::Token::Input(super::InputEvent {
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
                            sender.send(crate::token::Token::Input(super::InputEvent {
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
                            sender.send(crate::token::Token::Input(super::InputEvent {
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
                    sender.send(crate::token::Token::Input(super::InputEvent {
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
