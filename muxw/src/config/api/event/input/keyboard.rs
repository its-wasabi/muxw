// TODO: Use KeyboardCriteria type for passing name seat port

pub fn create_event_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let event_input_keyboard_table =
        lua.create_table()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.input.keyboard table",
            })?;

    event_input_keyboard_table
        .set(
            "added",
            lua.create_userdata(KeyboardEvent::Added(KeyboardDeviceEventContext::default()))
                .map_err(|err| crate::error::InitError::Mlua {
                    action: "create Mux.event.input.keyboard.added tag",
                })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.added tag",
        })?;

    event_input_keyboard_table
        .set(
            "removed",
            lua.create_userdata(KeyboardEvent::Removed(KeyboardDeviceEventContext::default()))
                .map_err(|err| crate::error::InitError::Mlua {
                    action: "crate Mux.event.input.keyboard.removed tag",
                })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.remove tag",
        })?;

    event_input_keyboard_table
        .set(
            "pressed",
            lua.create_userdata(KeyboardEvent::Pressed(KeyboardKeyEventContext::default()))
                .map_err(|err| crate::error::InitError::Mlua {
                    action: "create Mux.event.input.keyboard.pressed tag",
                })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.pressed tag",
        })?;

    event_input_keyboard_table
        .set(
            "keyrepeat",
            lua.create_userdata(KeyboardEvent::KeyRepeat(KeyboardKeyEventContext::default()))
                .map_err(|err| crate::error::InitError::Mlua {
                    action: "create Mux.event.input.keyboard.keyrepeat tag",
                })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard.keyrepeat tag",
        })?;

    Ok(event_input_keyboard_table)
}

#[derive(Debug, Clone)]
pub enum KeyboardEvent {
    Added {
        // TODO: While working with callback of keyboard added you are still using the keyboard
        // criteria (via KeyboardConfigBuilder), you should use only KeyboardConfig and apply
        // directly to kb object stored or referenced by the passed event to fire
        // 1. Swap KeyboardConfigBuilder to KeyboardConfig
        // 2. Find a way to store data used by the event but not exposed to the user
        // ! Remember to still update the Config struct in case of future updates
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
}

impl Eq for KeyboardEvent {}
impl PartialEq for KeyboardEvent {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl std::hash::Hash for KeyboardEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl mlua::UserData for KeyboardEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        match Self {}
    }
}

impl KeyboardEvent {
    pub fn into_lua_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        let t = lua.create_table()?;
        match self {
            KeyboardEvent::Added(ctx) | KeyboardEvent::Removed(ctx) => {
                t.set("", ctx.name.clone())?;
            }
            KeyboardEvent::Pressed(ctx) | KeyboardEvent::KeyRepeat(ctx) => {
                t.set("name", ctx.device.name.clone())?;
                t.set("seat", ctx.device.seat.clone())?;
                t.set("port", ctx.device.port.clone())?;
                t.set("keycode", ctx.keycode)?;
                t.set("keysym", ctx.keysym)?;
                t.set("utf8", ctx.utf8.clone())?;
            }
        }
        Ok(t)
    }
}
