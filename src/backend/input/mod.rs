// TODO: Refactor

use input::event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent};
use std::os::fd::{AsFd, AsRawFd};

mod xkb_manager;

const DEFAULT_DEVICES_CAPACITY: usize = 8;

pub struct Input {
    handle: crate::event_loop::IoHandle<crate::token::Token>,
    libinput: input::Libinput,
    devices: slab::Slab<input::Device>,
    device_keys: std::collections::HashMap<input::Device, DeviceKey>,
    emitter: crate::event_loop::Emitter<crate::token::Token>,
}

impl Input {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
        seat_handle: crate::backend::seat::SeatHandle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut libinput = input::Libinput::new_with_udev(LibinputInterface {
            seat_handle,
            devices: std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY),
        });

        libinput.udev_assign_seat("seat0").map_err(|()| {
            Box::<dyn std::error::Error>::from("Failed to assign udev seat: seat0")
        })?;

        let handle = event_loop.register_fd(
            &libinput.as_fd(),
            crate::event_loop::Interest::Readable,
            crate::token::Token::Input(Event::Dispatch),
        )?;

        Ok(Self {
            libinput,
            devices: slab::Slab::with_capacity(DEFAULT_DEVICES_CAPACITY),
            device_keys: std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY),
            emitter: event_loop.emitter(),
            handle,
        })
    }

    pub fn suspend(&self) {
        self.libinput.suspend();
    }

    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.libinput
            .resume()
            // TODO: Write real error
            .map_err(|()| Box::<dyn std::error::Error>::from("Failed to resume libinput"))
    }

    pub fn dispatch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.libinput.dispatch()?;
        for event in &mut self.libinput {
            match event {
                input::Event::Device(input::event::DeviceEvent::Added(device_add_event)) => {
                    let device = device_add_event.device();
                    let key = DeviceKey(self.devices.insert(device.clone()));
                    self.device_keys.insert(device, key);
                    self.emitter.emit(crate::token::Token::Input(Event::Device {
                        key,
                        kind: EventKind::DeviceAdded,
                    }));
                }

                input::Event::Device(input::event::DeviceEvent::Removed(device_remove_event)) => {
                    let device = device_remove_event.device();
                    if let Some(key) = self.device_keys.remove(&device) {
                        self.devices.remove(key.get());
                        self.emitter.emit(crate::token::Token::Input(Event::Device {
                            key,
                            kind: EventKind::DeviceRemoved,
                        }));
                    }
                }

                input::Event::Keyboard(keyboard_event) => {
                    let device = keyboard_event.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(Event::Device {
                            key: *key,
                            kind: EventKind::Keyboard {
                                keycode: keyboard_event.key(),
                                state: keyboard_event.key_state(),
                            },
                        }));
                    }
                }

                input::Event::Pointer(input::event::PointerEvent::Button(pointer_button_event)) => {
                    let device = pointer_button_event.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(Event::Device {
                            key: *key,
                            kind: EventKind::PointerButton {
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
                                self.emitter.emit(crate::token::Token::Input(Event::Device {
                                    key: *key,
                                    kind: EventKind::PointerVerticalScroll { scroll: vertical },
                                }));
                            }
                        }

                        if pointer_scroll_wheel_event
                            .has_axis(input::event::pointer::Axis::Horizontal)
                        {
                            let horizontal = pointer_scroll_wheel_event
                                .scroll_value(input::event::pointer::Axis::Horizontal);

                            if horizontal != 0.0 {
                                self.emitter.emit(crate::token::Token::Input(Event::Device {
                                    key: *key,
                                    kind: EventKind::PointerHorizontalScroll { scroll: horizontal },
                                }));
                            }
                        }
                    }
                }

                input::Event::Pointer(input::event::PointerEvent::Motion(motion)) => {
                    let device = motion.device();
                    if let Some(key) = self.device_keys.get(&device) {
                        self.emitter.emit(crate::token::Token::Input(Event::Device {
                            key: *key,
                            kind: EventKind::Motion {
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

struct LibinputInterface {
    seat_handle: crate::backend::seat::SeatHandle,
    devices: std::collections::HashMap<std::os::fd::RawFd, crate::backend::seat::DeviceId>,
}

impl input::LibinputInterface for LibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        _flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        let device = self.seat_handle.open_device(path).map_err(|error| {
            error.raw_os_error().unwrap_or_else(|| {
                tracing::warn!(
                    ?error,
                    "seat open_device error had no OS errno; reporting EIO"
                );
                rustix::io::Errno::IO.raw_os_error()
            })
        })?;

        self.devices.insert(device.fd.as_raw_fd(), device.id);
        Ok(device.fd)
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        if let Some(device_id) = self.devices.remove(&fd.as_raw_fd()) {
            if let Err(error) = self.seat_handle.close_device(device_id) {
                tracing::error!(?error, "Failed to close input device in libseat");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Dispatch,
    Device { key: DeviceKey, kind: EventKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceKey(usize);

impl DeviceKey {
    pub const fn get(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind {
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
