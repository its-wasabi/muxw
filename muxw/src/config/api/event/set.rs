pub fn create_event_set_function(
    lua: &mlua::Lua,
) -> Result<mlua::Function, crate::error::InitError> {
    let event_set_function = lua
        .create_function(
            |lua, (event_type, callback): (super::EventType, mlua::Function)| {
                here!("callback create call");
                Ok(())
            },
        )
        .map_err(|_| crate::error::InitError::Mlua { action: "todo!()" })?;

    Ok(event_set_function)
}
