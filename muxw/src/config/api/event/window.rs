pub fn create_event_window_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let event_window_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "crate Mlua.event.window table",
        })?;

    event_window_table
        .set(
            "mouse_enter",
            lua.create_userdata(WindowEventKind::MouseEnter(WindowMouseEventContext {}))
                .map_err(|err| crate::error::InitError::Mlua {
                    action: "create Mux.event.window.mouse_over tag",
                })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.window.mouse_over tag",
        })?;

    Ok(event_window_table)
}

#[derive(Debug, Clone)]
pub enum WindowEventKind {
    MouseEnter(WindowMouseEventContext),
}

impl Eq for WindowEventKind {}
impl PartialEq for WindowEventKind {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}
impl std::hash::Hash for WindowEventKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl WindowEventKind {
    pub fn into_lua_table(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        let t = lua.create_table()?;
        match self {
            Self::MouseEnter(ctx) => t.set("hello", 12),
        };

        Ok(t)
    }
}

impl mlua::UserData for WindowEventKind {}

#[derive(Debug, Clone)]
pub struct WindowMouseEventContext {}
