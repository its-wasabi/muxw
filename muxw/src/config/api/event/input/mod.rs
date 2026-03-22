pub mod keyboard;

pub fn create_event_input_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let t = lua
        .create_table()
        .map_err(|_| crate::error::InitError::Mlua {
            action: "create Mux.event.input table",
        })?;

    t.set(
        "keyboard",
        keyboard::create_event_input_keyboard_table(lua)?,
    )
    .map_err(|_| crate::error::InitError::Mlua {
        action: "set Mux.event.input.keyboard table",
    })?;

    Ok(t)
}

#[derive(Debug, Clone)]
pub enum InputEventKind {
    Keyboard(keyboard::InputKeyboardEventKind),
}

impl InputEventKind {
    pub fn matches(&self, tag: &Self) -> bool {
        match (self, tag) {
            (InputEventKind::Keyboard(fired), InputEventKind::Keyboard(tag)) => fired.matches(tag),
        }
    }

    pub fn populate_event_table(&self, t: &mlua::Table, lua: &mlua::Lua) -> mlua::Result<()> {
        match self {
            InputEventKind::Keyboard(kind) => kind.populate_event_table(t, lua),
        }
    }
}
