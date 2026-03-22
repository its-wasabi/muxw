// TODO: Use KeyboardCriteria type for passing name seat port

use crate::config::api::input::keyboard::KeyboardConfigBuilder;

pub fn create_event_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let event_input_keyboard_table =
        lua.create_table()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.input.keyboard table",
            })?;

    let tag = |kind: InputKeyboardEventKind, err_msg| {
        lua.create_userdata(crate::config::api::event::EventKind::Input(
            super::InputEventKind::Keyboard(kind),
        ))
        .map_err(|_| crate::error::InitError::Mlua { action: err_msg })
    };

    event_input_keyboard_table
        .set(
            "added",
            tag(
                InputKeyboardEventKind::Added {
                    keyboard: KeyboardConfigBuilder::default(),
                    location: muxw_types::input::DeviceLocation::default(),
                },
                "create Mux.event.input.keyboard.added tag",
            )?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.added tag",
        })?;

    event_input_keyboard_table
        .set(
            "removed",
            tag(
                InputKeyboardEventKind::Removed {
                    location: muxw_types::input::DeviceLocation::default(),
                },
                "create Mux.event.input.keyboard.removed tag",
            )?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.removed tag",
        })?;

    event_input_keyboard_table
        .set(
            "pressed",
            tag(
                InputKeyboardEventKind::Pressed {
                    location: muxw_types::input::DeviceLocation::default(),
                    key: muxw_types::input::keyboard::Key::default(),
                },
                "create Mux.event.input.keyboard.pressed tag",
            )?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.pressed tag",
        })?;

    event_input_keyboard_table
        .set(
            "keyrepeat",
            tag(
                InputKeyboardEventKind::KeyRepeat {
                    location: muxw_types::input::DeviceLocation::default(),
                    key: muxw_types::input::keyboard::Key::default(),
                },
                "create Mux.event.input.keyboard.keyrepeat tag",
            )?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.keyrepeat tag",
        })?;

    event_input_keyboard_table
        .set(
            "Inactivity",
            tag(
                InputKeyboardEventKind::Inactivity {
                    timeout_secs: 0,
                    keyboard: KeyboardConfigBuilder::default(),
                    location: muxw_types::input::DeviceLocation::default(),
                },
                "create Mux.event.input.keyboard.inactivity tag",
            )?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.inactivity tag",
        })?;

    Ok(event_input_keyboard_table)
}

#[derive(Debug, Clone)]
pub enum InputKeyboardEventKind {
    Added {
        // IMPORTANT: That is right make it in the way that .get() function for keyboard returns
        // keyboard builder with already linked found keyboard instead of just specifying criteria
        // TODO: While working with callback of keyboard added you are still using the keyboard
        // criteria (via KeyboardConfigBuilder), you should use only KeyboardConfig and apply
        // directly to kb object stored or referenced by the passed event to fire
        // 1. Swap KeyboardConfigBuilder to KeyboardConfig
        // 2. Find a way to store data used by the event but not exposed to the user
        // ! Remember to still update the Config struct in case of future updates
        // IMPORTANT: Remember to do that, later it will be only harder to refactor
        keyboard: crate::config::api::input::keyboard::KeyboardConfigBuilder,
        location: muxw_types::input::DeviceLocation,
    },
    Removed {
        location: muxw_types::input::DeviceLocation,
    },

    Pressed {
        location: muxw_types::input::DeviceLocation,
        key: muxw_types::input::keyboard::Key,
    },
    KeyRepeat {
        location: muxw_types::input::DeviceLocation,
        key: muxw_types::input::keyboard::Key,
    },

    Inactivity {
        timeout_secs: u64,
        keyboard: crate::config::api::input::keyboard::KeyboardConfigBuilder,
        location: muxw_types::input::DeviceLocation,
    },
}

impl InputKeyboardEventKind {
    pub fn matches(&self, other: &Self) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return false;
        }

        match (self, other) {
            (
                InputKeyboardEventKind::Inactivity {
                    timeout_secs: self_timeout_secs,
                    ..
                },
                InputKeyboardEventKind::Inactivity {
                    timeout_secs: other_timeout_secs,
                    ..
                },
            ) => self_timeout_secs == other_timeout_secs,
            _ => true,
        }
    }

    pub fn populate_event_table(&self, event: &mlua::Table, lua: &mlua::Lua) -> mlua::Result<()> {
        match self {
            InputKeyboardEventKind::Added { keyboard, location } => {
                event.set("keyboard", keyboard.clone())?;
                event.set("location", location.clone())?;
            }
            InputKeyboardEventKind::Removed { location } => {
                event.set("location", location.clone())?;
            }
            InputKeyboardEventKind::Pressed { location, key } => {
                event.set("location", location.clone())?;
                event.set("key", key.clone())?;
            }
            InputKeyboardEventKind::KeyRepeat { location, key } => {
                event.set("location", location.clone())?;
                event.set("key", key.clone())?;
            }
            InputKeyboardEventKind::Inactivity {
                timeout_secs,
                location,
                keyboard,
            } => {
                event.set("timeout_secs", *timeout_secs)?;
                event.set("location", location.clone())?;
                event.set("keyboard", keyboard.clone())?;
            }
        }
        Ok(())
    }
}
