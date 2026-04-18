pub mod keyboard;

pub fn create_event_input_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let t = lua
        .create_table()
        .map_err(|_| crate::error::InitError::Mlua {
            action: "create Mux.event.input table",
        })?;

    here!("KB ADDED");

    t.set(
        "keyboard",
        keyboard::create_event_input_keyboard_table(lua)?,
    )
    .map_err(|_| crate::error::InitError::Mlua {
        action: "set Mux.event.input.keyboard table",
    })?;

    here!("AND OUT");

    Ok(t)
}
