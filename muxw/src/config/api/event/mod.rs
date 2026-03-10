pub mod input;

use mlua::LuaSerdeExt;

pub fn create_event_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let event_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.event table",
        })?;

    event_table
        .set("input", input::create_event_input_table(lua)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input table",
        })?;

    event_table
        .set(
            "add",
            lua.create_function(
                move |lua, (event_kind, callback): (EventKind, mlua::Function)| {
                    lua.app_data_mut::<EventManager>().unwrap().register(
                        lua,
                        Event {
                            kind: event_kind,
                            timestamp: std::time::Instant::now(),
                        },
                        callback,
                    );
                    Ok(())
                },
            )
            .unwrap(),
        )
        .unwrap();

    Ok(event_table)
}

pub struct Event {
    kind: EventKind,
    timestamp: std::time::Instant,
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self {
            kind,
            timestamp: std::time::Instant::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub enum EventKind {
    Keyboard(input::keyboard::KeyboardEvent),
}

impl mlua::FromLua for EventKind {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        lua.from_value(value)
    }
}

#[derive(Default)]
pub struct EventManager {
    registry: std::collections::HashMap<EventKind, Vec<mlua::RegistryKey>>,
}

impl EventManager {
    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        event: Event,
        callback: mlua::Function,
    ) -> mlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        self.registry.entry(event.kind).or_default().push(key);
        Ok(())
    }
    pub fn call(&self, lua: &mlua::Lua, event: &Event) -> mlua::Result<()> {
        if let Some(keys) = self.registry.get(&event.kind) {
            for key in keys {
                let callback: mlua::Function = lua.registry_value(key)?;
                callback.call::<()>(())?;
            }
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.registry.clear();
    }
}
