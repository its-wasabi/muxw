pub mod keyboard;

pub fn create_input_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let input_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.input table",
        })?;

    input_table
        .set("keyboard", keyboard::create_input_keyboard_table(lua)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.input.keyboard table",
        })?;

    // TODO: Implement rest of the tables

    Ok(input_table)
}
