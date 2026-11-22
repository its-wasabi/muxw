use eyre::Context;
use input::{
    AsRaw as _,
    event::{
        EventTrait,
        keyboard::KeyboardEventTrait,
        pointer::{PointerEventTrait, PointerScrollEvent},
    },
};
use slotmap::Key;
use std::os::fd::{AsFd as _, AsRawFd as _};

const DEFAULT_DEVICES_CAPACITY: usize = 2;

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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq,Eq)]
    pub struct Capabilities: u8 {
        const KEYBOARD = 1 << 0;
        const POINTER = 1 << 1;
        const TOUCH = 1 << 2;
        const GESTURE = 1 << 3;
        const TABLET_TOOL = 1 << 4;
        const TABLET_PAD = 1 << 5;
        const SWITCH = 1 << 6;
    }
}

impl Capabilities {
    #[inline]
    fn from_raw(device: &input::Device) -> Self {
        use input::DeviceCapability as C;
        let mut caps = Self::empty();
        if device.has_capability(C::Keyboard) {
            caps |= Self::KEYBOARD;
        }
        if device.has_capability(C::Pointer) {
            caps |= Self::POINTER;
        }
        if device.has_capability(C::Touch) {
            caps |= Self::TOUCH;
        }
        if device.has_capability(C::Gesture) {
            caps |= Self::GESTURE;
        }
        if device.has_capability(C::TabletTool) {
            caps |= Self::TABLET_TOOL;
        }
        if device.has_capability(C::TabletPad) {
            caps |= Self::TABLET_PAD;
        }
        if device.has_capability(C::Switch) {
            caps |= Self::SWITCH;
        }
        caps
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventKind<'a> {
    DeviceAdded {
        name: &'a str,
        sysname: &'a str,
        capabilities: Capabilities,
    },
    DeviceRemoved {
        capabilities: Capabilities,
    },

    KeyboardKey {
        keycode: u32,
        pressed: bool,
        time_msec: u32,
    },

    PointerButton {
        keycode: u32,
        pressed: bool,
        time_msec: u32,
    },
    PointerScroll {
        horizontal: Option<f64>,
        vertical: Option<f64>,
        time_msec: u32,
    },
    PointerMotion {
        dx: f64,
        dy: f64,
        dx_unaccel: f64,
        dy_unaccel: f64,
        time_usec: u32,
    },
    PointerMotionAbsolute {
        x: f64,
        y: f64,
        time_msec: u32,
    },
}

pub struct InputEvent<'a> {
    pub key: DeviceKey,
    pub kind: EventKind<'a>,
}

pub struct Libinput {
    handle: evl::IoHandle<crate::Event>,
    libinput: input::Libinput,
    devices: slotmap::SlotMap<DeviceKey, input::Device>,
}

impl Libinput {
    pub fn new(
        event_loop: &mut evl::EventLoop<crate::Event>,
        seat_handle: crate::backend::session::SeatHandle,
    ) -> eyre::Result<Self> {
        let mut libinput = input::Libinput::new_with_udev(LibinputInterface {
            seat_handle,
            devices: std::collections::HashMap::with_capacity(DEFAULT_DEVICES_CAPACITY),
        });

        libinput
            .udev_assign_seat("seat0")
            .map_err(|()| eyre::eyre!("failed to assign udev seat 'seat0' to libinput"))?;

        let handle = event_loop
            .register_fd(
                &libinput.as_fd(),
                evl::Interest::Readable,
                crate::Event::Input(Event::Dispatch),
            )
            .wrap_err("failed to register libinput fd in event loop")?;

        Ok(Self {
            libinput,
            devices: slotmap::SlotMap::with_capacity_and_key(DEFAULT_DEVICES_CAPACITY),
            handle,
        })
    }

    pub fn suspend(&self) {
        self.libinput.suspend();
    }

    pub fn resume(&mut self) -> eyre::Result<()> {
        self.libinput
            .resume()
            .map_err(|()| eyre::eyre!("failed to resume libinput"))
    }

    pub fn dispatch(&mut self, mut on_event: impl FnMut(InputEvent)) -> eyre::Result<()> {
        self.libinput
            .dispatch()
            .wrap_err("failed to dispatch libinput events")?;

        for event in &mut self.libinput {
            match event {
                input::Event::Device(input::event::DeviceEvent::Added(event)) => {
                    let device = event.device();
                    let key = self.devices.insert(device.clone());
                    set_key(&device, key);

                    let kind = EventKind::DeviceAdded {
                        name: &device.name(),
                        sysname: &device.sysname(),
                        capabilities: Capabilities::from_raw(&device),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Device(input::event::DeviceEvent::Removed(event)) => {
                    let device = event.device();
                    let key = get_key(&device);
                    self.devices.remove(key);

                    let kind = EventKind::DeviceRemoved {
                        capabilities: Capabilities::from_raw(&device),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Keyboard(event) => {
                    let device = event.device();
                    let key = get_key(&device);

                    let pressed = event.key_state() == input::event::keyboard::KeyState::Pressed;
                    let kind = EventKind::KeyboardKey {
                        keycode: event.key(),
                        pressed,
                        time_msec: event.time(),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::Button(event)) => {
                    let device = event.device();
                    let key = get_key(&device);

                    let button_state = event.button_state();
                    let pressed = button_state == input::event::pointer::ButtonState::Pressed;
                    let kind = EventKind::PointerButton {
                        keycode: event.button(),
                        pressed,
                        time_msec: event.time(),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::ScrollWheel(event)) => {
                    let device = event.device();
                    let key = get_key(&device);

                    let horizontal = if event.has_axis(input::event::pointer::Axis::Horizontal) {
                        Some(event.scroll_value(input::event::pointer::Axis::Horizontal))
                    } else {
                        None
                    };
                    let vertical = if event.has_axis(input::event::pointer::Axis::Vertical) {
                        Some(event.scroll_value(input::event::pointer::Axis::Vertical))
                    } else {
                        None
                    };

                    let kind = EventKind::PointerScroll {
                        horizontal,
                        vertical,
                        time_msec: event.time(),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::Motion(event)) => {
                    let device = event.device();
                    let key = get_key(&device);

                    let kind = EventKind::PointerMotion {
                        dx: event.dx(),
                        dy: event.dy(),
                        dx_unaccel: event.dx_unaccelerated(),
                        dy_unaccel: event.dy_unaccelerated(),
                        time_usec: event.time(),
                    };

                    on_event(InputEvent { key, kind });
                }

                input::Event::Pointer(input::event::PointerEvent::MotionAbsolute(event)) => {
                    let device = event.device();
                    let key = get_key(&device);

                    let kind = EventKind::PointerMotionAbsolute {
                        x: event.absolute_x_transformed(1),
                        y: event.absolute_y_transformed(1),
                        time_msec: event.time(),
                    };

                    on_event(InputEvent { key, kind });
                }

                _ => {}
            }
        }

        Ok(())
    }
}

struct LibinputInterface {
    seat_handle: crate::backend::session::SeatHandle,
    devices: std::collections::HashMap<std::os::fd::RawFd, crate::backend::session::DeviceId>,
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

#[inline]
fn set_key(device: &input::Device, key: DeviceKey) {
    unsafe {
        input::ffi::libinput_device_set_user_data(device.as_raw().cast_mut(), key.into());
    }
}

#[inline]
fn get_key(device: &input::Device) -> DeviceKey {
    unsafe { input::ffi::libinput_device_get_user_data(device.as_raw().cast_mut()).into() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Dispatch,
}

impl Into<crate::Event> for Event {
    fn into(self) -> crate::Event {
        crate::Event::Input(self)
    }
}
