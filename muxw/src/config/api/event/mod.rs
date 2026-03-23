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
                move |lua, (tag, callback): (mlua::AnyUserData, mlua::Function)| {
                    let tag = tag.borrow::<EventTag>()?;
                    let mut registry = lua
                        .app_data_mut::<EventRegistry>()
                        .ok_or_else(|| mlua::Error::runtime("Registry not initialized"))?;
                    let id = registry.register(lua, tag.0.clone_box(), callback)?;

                    Ok(id)
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

#[derive(Default)]
pub struct EventRegistry {
    next_id: EventId,
    entries: std::collections::HashMap<EventId, RegisteredEvent>,
}

struct RegisteredEvent {
    tag: Box<dyn ErasedEventKind>,
    callback_key: mlua::RegistryKey,
}

pub type EventId = u64;

pub struct EventTag(pub Box<dyn ErasedEventKind>);

pub trait ErasedEventKind: Send + Sync {
    fn matches(&self, other: &dyn ErasedEventKind) -> bool;
    fn populate_event_table(&self, event: &mlua::Table, lua: &mlua::Lua) -> mlua::Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_box(&self) -> Box<dyn ErasedEventKind>;
    fn is_callable(&self) -> bool {
        false
    }
    fn call_with_args(&self, lua: &mlua::Lua, args: mlua::MultiValue) -> mlua::Result<EventTag>;
}

impl EventRegistry {
    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        tag: Box<dyn ErasedEventKind>,
        callback: mlua::Function,
    ) -> mlua::Result<EventId> {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(
            id,
            RegisteredEvent {
                tag,
                callback_key: lua.create_registry_value(callback)?,
            },
        );

        Ok(id)
    }

    pub fn fire(
        &self,
        lua: &mlua::Lua,
        event: &dyn ErasedEventKind,
        timestamp: f64,
    ) -> mlua::Result<()> {
        for entry in self.entries.values() {
            // TODO: check why calling .as_ref() additionally removed error
            if event.matches(entry.tag.as_ref()) {
                let event_table = lua.create_table()?;
                event_table.set("timestamp", timestamp)?;
                event.populate_event_table(&event_table, lua)?;
                let callback: mlua::Function = lua.registry_value(&entry.callback_key)?;
                callback.call::<()>(event_table)?;
            }
        }

        Ok(())
    }

    pub fn unregister(&mut self, lua: &mlua::Lua, id: EventId) -> mlua::Result<()> {
        if let Some(entry) = self.entries.remove(&id) {
            lua.remove_registry_value(entry.callback_key)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl mlua::UserData for EventTag {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {}
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::Call,
            |lua, this, args: mlua::MultiValue| this.0.call_with_args(lua, args),
        );
    }
}
