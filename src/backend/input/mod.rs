mod error;

use input::{
    AsRaw,
    event::{EventTrait, keyboard::KeyboardEventTrait, pointer::PointerScrollEvent},
};
use slotmap::Key;
use std::os::fd::{AsFd, AsRawFd};

const DEFAULT_DEVICES_CAPACITY: usize = 8;

slotmap::new_key_type! {
    pub struct DeviceKey;
}

impl From<*mut std::ffi::c_void> for DeviceKey {
    #[inline]
    fn from(val: *mut std::ffi::c_void) -> Self {
        slotmap::KeyData::from_ffi(val as u64).into()
    }
}

impl From<DeviceKey> for *mut std::ffi::c_void {
    #[inline]
    fn from(value: DeviceKey) -> Self {
        value.data().as_ffi() as *mut std::ffi::c_void
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
    PointerScroll {
        horizontal: f64,
        vertical: f64,
    },
    Motion {
        delta_x: f64,
        delta_y: f64,
    },
}

pub struct InputEvent {
    pub key: DeviceKey,
    pub kind: EventKind,
}

pub struct Input {
    handle: crate::event_loop::IoHandle<crate::token::Token>,
    libinput: input::Libinput,
    devices: slotmap::SlotMap<DeviceKey, input::Device>,
}

impl Input {
    pub fn new(
        event_loop: &mut crate::event_loop::EventLoop<crate::token::Token>,
        seat_handle: crate::backend::seat::SeatHandle,
    ) -> Result<Self, error::Error> {
        let mut libinput = input::Libinput::new_with_udev(LibinputInterface {
            seat_handle,
            devices: std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY),
        });

        libinput
            .udev_assign_seat("seat0")
            .map_err(|()| error::Error::AssignSeat)?;

        let handle = event_loop.register_fd(
            &libinput.as_fd(),
            crate::event_loop::Interest::Readable,
            crate::token::Token::Input(Event::Dispatch),
        )?;

        Ok(Self {
            libinput,
            devices: slotmap::SlotMap::with_capacity_and_key(DEFAULT_DEVICES_CAPACITY),
            handle,
        })
    }

    pub fn suspend(&self) {
        self.libinput.suspend();
    }

    pub fn resume(&mut self) -> Result<(), error::Error> {
        self.libinput.resume().map_err(|()| error::Error::Resume)
    }

    pub fn dispatch(&mut self, triggered: &mut Vec<InputEvent>) -> Result<(), error::Error> {
        triggered.clear();
        self.libinput.dispatch()?;

        for event in &mut self.libinput {
            match event {
                input::Event::Device(input::event::DeviceEvent::Added(event)) => {
                    let device = event.device();
                    let key = self.devices.insert(device.clone());
                    unsafe {
                        input::ffi::libinput_device_set_user_data(
                            device.as_raw().cast_mut(),
                            key.into(),
                        );
                    }

                    let kind = EventKind::DeviceAdded;

                    triggered.push(InputEvent { key, kind });
                }

                input::Event::Device(input::event::DeviceEvent::Removed(event)) => {
                    let device = event.device();
                    let key = unsafe {
                        input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into()
                    };
                    self.devices.remove(key);

                    let kind = EventKind::DeviceRemoved;

                    triggered.push(InputEvent { key, kind });
                }

                input::Event::Keyboard(event) => {
                    let device = event.device();
                    let key = unsafe {
                        input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into()
                    };

                    let kind = EventKind::Keyboard {
                        keycode: event.key(),
                        state: event.key_state(),
                    };

                    triggered.push(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::Button(event)) => {
                    let device = event.device();
                    let key = unsafe {
                        input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into()
                    };

                    let kind = EventKind::PointerButton {
                        keycode: event.button(),
                        state: event.button_state(),
                    };

                    triggered.push(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::ScrollWheel(event)) => {
                    let device = event.device();
                    let key = unsafe {
                        input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into()
                    };

                    let kind = EventKind::PointerScroll {
                        horizontal: event.scroll_value(input::event::pointer::Axis::Horizontal),
                        vertical: event.scroll_value(input::event::pointer::Axis::Vertical),
                    };

                    triggered.push(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::Motion(event)) => {
                    let device = event.device();
                    let key = unsafe {
                        input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into()
                    };

                    let kind = EventKind::Motion {
                        delta_x: event.dx(),
                        delta_y: event.dy(),
                    };

                    triggered.push(InputEvent { key, kind });
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
        let device = self.seat_handle.open_device_raw(path)?;
        self.devices.insert(device.fd.as_raw_fd(), device.id);
        Ok(device.fd)
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        if let Some(device_id) = self.devices.remove(&fd.as_raw_fd())
            && let Err(error) = self.seat_handle.close_device(device_id)
        {
            tracing::error!(?error, "Failed to close input device in libseat");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Dispatch,
}
