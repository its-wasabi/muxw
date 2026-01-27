pub mod keyboard;

pub fn create_input_table(
    lua: &mlua::Lua,
    state: &crate::config::ConfigState,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Ray.input table",
            source: err,
        })?;

    input_table
        .set(
            "keyboard",
            keyboard::create_input_keyboard_table(lua, state)?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Ray.input.keyboard table",
            source: err,
        })?;

    // TODO: Implement rest of the tables

    Ok(input_table)
}
