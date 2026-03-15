#![allow(clippy::todo)]
#![allow(clippy::unwrap_used)]

use crate::config::api::event;

pub mod input;
pub mod window;

pub fn create_event_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let event_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.event table",
        })?;

    event_table
        .set(
            "add",
            lua.create_function(
                move |lua, (event_kind, callback): (Event, mlua::Function)| {
                    todo!("Register event");
                    todo!("Return event listener id");
                    Ok(())
                },
            )
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.add function",
            })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.add function",
        });

    event_table
        .set("input", input::create_event_input_table(lua)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.input table",
        })?;

    event_table
        .set("window", window::create_event_window_table(lua)?)
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.window table",
        })?;

    Ok(event_table)
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Event {
    Input(input::InputEvent),
    Window(window::WindowEventKind),
}

impl mlua::FromLua for Event {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let mlua::Value::UserData(ref event_kind) = value else {
            todo!()
        };

        if let Ok(kind) = event_kind.borrow::<input::InputEvent>() {
            return Ok(Self::Input(kind.clone()));
        }
        if let Ok(kind) = event_kind.borrow::<window::WindowEventKind>() {
            return Ok(Self::Window(kind.clone()));
        }

        Err(mlua::Error::runtime("unknown event type token"))
    }
}

impl Event {
    fn into_lua(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        match self {
            Self::Input(event) => Ok(event.into_lua_table(lua)?),
            Self::Window(event) => Ok(event.into_lua_table(lua)?),
        }
    }
}

#[derive(Default)]
pub struct EventRegistry {
    registry: std::collections::HashMap<Event, Vec<mlua::RegistryKey>>,
}

impl EventRegistry {
    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        kind: Event,
        callback: mlua::Function,
    ) -> mlua::Result<()> {
        Ok(self
            .registry
            .entry(kind)
            .or_default()
            .push(lua.create_registry_value(callback)?))
    }
    pub fn fire(&self, lua: &mlua::Lua, kind: &Event) -> mlua::Result<()> {
        let Some(keys) = self.registry.get(&kind) else {
            return Ok(());
        };

        let ctx = kind.into_lua(lua)?;

        here!("Fire ({}) kind: {keys:?}", keys.len());

        for key in keys {
            let callback: mlua::Function = lua.registry_value(key)?;
            // FIXME: Returns error when called
            callback.call::<()>(ctx.clone())?;
        }

        Ok(())
    }
    pub fn clear(&mut self) {
        self.registry.clear();
    }
}
