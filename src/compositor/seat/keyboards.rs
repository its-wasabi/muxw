#![allow(unused)]

use rustix::net::eth::MOBITEX;

const MOD_MASK: xkbcommon::xkb::StateFlags = xkbcommon::xkb::STATE_MODS_DEPRESSED
    | xkbcommon::xkb::STATE_MODS_LATCHED
    | xkbcommon::xkb::STATE_MODS_LOCKED
    | xkbcommon::xkb::STATE_LAYOUT_EFFECTIVE;

pub(super) struct Keyboards {
    context: xkbcommon::xkb::Context,
    compose_table: Option<xkbcommon::xkb::compose::Table>,
    keyboards: slotmap::SecondaryMap<crate::backend::input::DeviceKey, KeyboardState>,
}

impl Keyboards {
    pub(super) fn new() -> Self {
        let context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_ENVIRONMENT_NAMES);

        let locale = std::env::var_os("LC_ALL")
            .or_else(|| std::env::var_os("LC_CTYPE"))
            .or_else(|| std::env::var_os("LANG"))
            .unwrap_or_else(|| std::ffi::OsString::from("C.UTF-8"));
        let compose_table = xkbcommon::xkb::compose::Table::new_from_locale(
            &context,
            &locale,
            xkbcommon::xkb::compose::COMPILE_NO_FLAGS,
        )
        .ok();

        Self {
            context,
            compose_table,

            keyboards: slotmap::SecondaryMap::new(),
        }
    }

    pub(super) fn add_device(
        &mut self,
        key: crate::backend::input::DeviceKey,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let keyboard_state = KeyboardState::new(self)?;
        self.keyboards.insert(key, keyboard_state);

        Ok(())
    }

    pub(super) fn remove_device(&mut self, key: crate::backend::input::DeviceKey) {
        self.keyboards.remove(key);
    }
}

struct KeyboardState {
    state: xkbcommon::xkb::State,
    compose_state: Option<xkbcommon::xkb::compose::State>,

    keymap: xkbcommon::xkb::Keymap,
}

impl KeyboardState {
    fn new(context: &Keyboards) -> Result<Self, Box<dyn std::error::Error>> {
        let keymap = xkbcommon::xkb::Keymap::new_from_names(
            &context.context,
            "",
            "",
            "",
            "",
            None,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        // TODO: Introduce real error handling
        .ok_or("Failed to build keymap")?;

        let state = xkbcommon::xkb::State::new(&keymap);
        let compose_state = context.compose_table.as_ref().map(|compose| {
            xkbcommon::xkb::compose::State::new(compose, xkbcommon::xkb::compose::STATE_NO_FLAGS)
        });

        Ok(Self {
            state,
            compose_state,

            keymap,
        })
    }

    fn process_keys(&mut self, keycode: u32, pressed: bool) -> KeyOutput {
        // NOTE: 8 is the difference between xkb and libinput keycodes
        let keycode = xkbcommon::xkb::Keycode::new(keycode + 8);
        let direction = if pressed {
            xkbcommon::xkb::KeyDirection::Down
        } else {
            xkbcommon::xkb::KeyDirection::Up
        };

        let keysym = self.state.key_get_one_sym(keycode);
        // TODO: Check if that requires branching
        let repeats = if pressed {
            self.keymap.key_repeats(keycode)
        } else {
            false
        };

        let changed = self.state.update_key(keycode, direction);
        let modifiers_changed = if (changed & MOD_MASK) != 0 {
            Some(Modifiers {
                depressed: self
                    .state
                    .serialize_mods(xkbcommon::xkb::STATE_MODS_DEPRESSED),
                latched: self
                    .state
                    .serialize_mods(xkbcommon::xkb::STATE_MODS_LATCHED),
                locked: self.state.serialize_mods(xkbcommon::xkb::STATE_MODS_LOCKED),
                layout: self
                    .state
                    .serialize_layout(xkbcommon::xkb::STATE_LAYOUT_EFFECTIVE),
            })
        } else {
            None
        };

        KeyOutput {
            keysym,
            repeats,
            modifiers_changed,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Modifiers {
    pub depressed: u32,
    pub latched: u32,
    pub locked: u32,
    pub layout: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyOutput {
    pub keysym: xkbcommon::xkb::Keysym,
    pub repeats: bool,
    pub modifiers_changed: Option<Modifiers>,
}
