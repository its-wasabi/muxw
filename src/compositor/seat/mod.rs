mod keyboards;
mod pointers;

pub(super) struct Seat {
    keyboards: keyboards::Keyboards,
    pointers: pointers::Pointers,
}

impl Seat {
    pub(super) fn new() -> Self {
        Self {
            keyboards: keyboards::Keyboards::new(),
            pointers: pointers::Pointers::new(),
        }
    }

    pub(super) fn process_input(
        &mut self,
        event: crate::backend::input::InputEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event.kind {
            crate::backend::input::EventKind::KeyboardKey {
                keycode,
                pressed,
                time_msec,
            } => todo!(),
            crate::backend::input::EventKind::PointerButton {
                keycode,
                pressed,
                time_msec,
            } => todo!(),
            crate::backend::input::EventKind::PointerScroll {
                horizontal,
                vertical,
                time_msec,
            } => todo!(),
            crate::backend::input::EventKind::PointerMotion {
                dx,
                dy,
                dx_unaccel,
                dy_unaccel,
                time_usec,
            } => todo!(),
            crate::backend::input::EventKind::PointerMotionAbsolute { x, y, time_msec } => todo!(),

            crate::backend::input::EventKind::DeviceAdded {
                name,
                sysname,
                capabilities,
            } => {
                if capabilities.contains(crate::backend::input::Capabilities::KEYBOARD) {
                    self.keyboards.add_device(event.key);
                }
            }
            crate::backend::input::EventKind::DeviceRemoved { capabilities } => {
                // NOTE: If config would bind on remove event handle it there
                self.keyboards.remove_device(event.key);
            }
        }

        Ok(())
    }
}
