pub mod input;

pub fn create_global_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let global_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create global table Mux",
        })?;

    global_table
        .set("input", input::create_input_table(lua)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.input table",
        });

    // TODO: Implement rest of the tables

    Ok(global_table)
}
