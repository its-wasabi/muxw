use std::any::Any;

pub mod keyboard;

pub fn create_event_input_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let event_input_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.event.input table",
        })?;

    event_input_table
        .set(
            "keyboard",
            keyboard::create_event_input_keyboard_table(lua)?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input.keyboard table",
        })?;

    Ok(event_input_table)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InputEventKind {
    Keyboard(keyboard::KeyboardEvent),
}

impl mlua::UserData for InputEventKind {}

impl InputEventKind {
    pub fn into_lua_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        let t = lua.create_table()?;
        match self {
            InputEventKind::Keyboard(ctx) => t.set("keyboard", ctx.into_lua_table(lua)?)?,
        }

        Ok(t)
    }
}
