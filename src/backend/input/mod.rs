use std::os::fd::{AsFd, AsRawFd};

use input::event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent};

mod worker;
mod xkb_manager;

pub struct InputManager {
    seat: String,
    libinput: input::Libinput,
    devices: slab::Slab<input::Device>,
    device_keys: std::collections::HashMap<input::Device, InputDeviceKey>,
    emitter: crate::event_loop::EventEmitter<crate::token::Token>,
}

impl InputManager {
    pub fn new(
        event_loop: &crate::event_loop::EventLoop<crate::token::Token>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let libinput = input::Libinput::new_with_udev(LibinputInterface { sender: todo!() });

        event_loop.register_source(
            &libinput.as_fd(),
            polling::PollMode::Edge,
            crate::token::Token::Libinput,
        )?;

        Ok(Self {
            seat: String::from("seat0"),
            libinput,
            devices: slab::Slab::with_capacity(1),
            device_keys: std::collections::HashMap::with_capacity(1),
            emitter: event_loop.emiter(),
        })
    }

    pub fn dispatch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.libinput.dispatch()?;
        for event in &mut self.libinput {
            match event {
                input::Event::Device(input::event::DeviceEvent::Added(device_add_event)) => {
                    let device = device_add_event.device();
                    let key = InputDeviceKey(self.devices.insert(device.clone()));
                    self.device_keys.insert(device, key);
                    self.emitter.emit(crate::token::Token::Input(InputEvent {
                        key,
                        kind: InputEventKind::DeviceAdded,
                    }));
                }

                input::Event::Device(input::event::DeviceEvent::Removed(device_remove_event)) => {
                    let device = device_remove_event.device();
                    if let Some(key) = self.device_keys.remove(&device) {
                        self.devices.remove(key.get());
                        self.emitter.emit(crate::token::Token::Input(InputEvent {
                            key,
                            kind: InputEventKind::DeviceRemoved,
                        }));
                    }
                }

                input::Event::Keyboard(keyboard_event) => {
                    let device = keyboard_event.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(InputEvent {
                            key: *key,
                            kind: InputEventKind::Keyboard {
                                keycode: keyboard_event.key(),
                                state: keyboard_event.key_state(),
                            },
                        }));
                    }
                }

                input::Event::Pointer(input::event::PointerEvent::Button(pointer_button_event)) => {
                    let device = pointer_button_event.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(InputEvent {
                            key: *key,
                            kind: InputEventKind::PointerButton {
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
                    if let Some(key) = self.device_keys.get(&device) {
                        if pointer_scroll_wheel_event
                            .has_axis(input::event::pointer::Axis::Vertical)
                        {
                            let vertical = pointer_scroll_wheel_event
                                .scroll_value(input::event::pointer::Axis::Vertical);

                            if vertical != 0.0 {
                                self.emitter.emit(crate::token::Token::Input(InputEvent {
                                    key: *key,
                                    kind: InputEventKind::PointerVerticalScroll {
                                        scroll: vertical,
                                    },
                                }));
                            }
                        }

                        if pointer_scroll_wheel_event
                            .has_axis(input::event::pointer::Axis::Horizontal)
                        {
                            let horizontal = pointer_scroll_wheel_event
                                .scroll_value(input::event::pointer::Axis::Horizontal);

                            if horizontal != 0.0 {
                                self.emitter.emit(crate::token::Token::Input(InputEvent {
                                    key: *key,
                                    kind: InputEventKind::PointerHorizontalScroll {
                                        scroll: horizontal,
                                    },
                                }));
                            }
                        }
                    }
                }

                input::Event::Pointer(input::event::PointerEvent::Motion(motion)) => {
                    let device = motion.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(InputEvent {
                            key: *key,
                            kind: InputEventKind::Motion {
                                delta_x: motion.dx(),
                                delta_y: motion.dy(),
                            },
                        }));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
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

        self.sender.send(crate::token::Token::Seat(
            crate::backend::seat::SeatEvent::OpenRequest(Box::new(
                crate::backend::seat::SeatOpenData {
                    path: path.to_path_buf(),
                    reply,
                },
            )),
        ));

        match response.recv() {
            Ok(Ok(fd)) => Ok(fd),
            Ok(Err(io_error)) => Err(io_error.raw_os_error().unwrap_or(13)),
            Err(_) => Err(13),
        }
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        let raw_fd = fd.as_raw_fd();
        std::mem::drop(fd);
        self.sender.send(crate::token::Token::Seat(
            crate::backend::seat::SeatEvent::CloseRequest(raw_fd),
        ));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputEvent {
    pub key: InputDeviceKey,
    pub kind: InputEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputDeviceKey(usize);

impl InputDeviceKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEventKind {
    DeviceAdded,
    DeviceRemoved,
    Keyboard {
        keycode: u32,
        state: input::event::keyboard::KeyState,
    },
    PointerButton {
        keycode: u32,
        state: input::event::pointer::ButtonState,
    },

    PointerVerticalScroll {
        scroll: f64,
    },

    PointerHorizontalScroll {
        scroll: f64,
    },

    Motion {
        delta_x: f64,
        delta_y: f64,
    },
}
