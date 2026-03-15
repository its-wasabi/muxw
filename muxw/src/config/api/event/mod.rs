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
                move |lua, (event_kind, callback): (EventKind, mlua::Function)| {
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

pub trait EventKind: 'static {
    type Context;
    fn into_lua_table(context: &Self::Context, lua: &mlua::Lua) -> mlua::Result<mlua::Table>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Event(dyn EventKind);

#[derive(Default)]
pub struct EventRegistry {
    registry: std::collections::HashMap<std::any::TypeId, Vec<mlua::RegistryKey>>,
}

impl EventRegistry {
    pub fn register(&mut self, lua: &mlua::Lua, callback: mlua::Function) -> mlua::Result<()> {
        Ok(self
            .registry
            .entry(kind)
            .or_default()
            .push(lua.create_registry_value(callback)?))
    }

    pub fn fire(&self, lua: &mlua::Lua, event: impl Event) -> mlua::Result<()> {
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
