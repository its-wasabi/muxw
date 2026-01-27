pub mod input;

pub fn create_global_table(
    lua: &mlua::Lua,
    state: &crate::config::ConfigState,
) -> Result<mlua::Table, crate::error::InitError> {
    let global_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create global table Ray",
            source: err,
        })?;

    global_table
        .set("input", input::create_input_table(lua, state)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Ray.input table",
            source: err,
        });

    // TODO: Implement rest of the tables

    Ok(global_table)
}
