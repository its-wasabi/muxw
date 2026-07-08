pub struct Seat {
    pub keyboards:
        std::collections::HashMap<crate::backend::input::InputDeviceKey, LogicalKeyboard>,
}

impl Seat {
    pub fn new() -> Self {
        Self {
            keyboards: std::collections::HashMap::with_capacity(1),
        }
    }

    pub fn add_keyboard(&mut self) {
        let ctx = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let keymap = xkbcommon::xkb::Keymap::new_from_names(
            &ctx,
            "",
            "",
            "",
            "",
            None,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();

        let xkb_state = xkbcommon::xkb::State::new(&keymap);

        // self.keyboards.insert()
    }

    pub fn remove_keyboard(&mut self, key: usize) {
        // self.keyboards.remove(key);
    }
}

pub struct LogicalKeyboard {
    pub xkb_state: xkbcommon::xkb::State,
    pub keymap: xkbcommon::xkb::Keymap,
}
