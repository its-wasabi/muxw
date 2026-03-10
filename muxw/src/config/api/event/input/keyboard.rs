use mlua::LuaSerdeExt;

pub fn create_event_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let event_input_keyboard_table =
        lua.create_table()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.input.keyboard table",
            })?;

    Ok(event_input_keyboard_table)
}

// TODO: Change name, seat, and port into single context type
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum KeyboardEvent {
    Added {
        name: String,
        seat: String,
        port: String,
    },

    Removed {
        name: String,
        seat: String,
        port: String,
    },

    Pressed {
        name: String,
        seat: String,
        port: String,

        keycode: u32,
        keysym: u32,
        utf8: Option<String>,
        context: (), // TODO: Make that contain keys pressed alongside
    },

    KeyRepeat {
        name: String,
        seat: String,
        port: String,

        keycode: u32,
        keysym: u32,
        utf8: Option<String>,
        context: (), // TODO: Make that contain keys pressed alongside
    },
}

impl mlua::FromLua for KeyboardEvent {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        lua.from_value(value)
    }
}
