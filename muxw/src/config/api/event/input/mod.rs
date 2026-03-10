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
