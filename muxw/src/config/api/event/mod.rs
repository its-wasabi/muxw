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
            lua.create_function(move |lua, (tag, callback): (mlua::Value, mlua::Function)| {
                let tag = match tag {
                    mlua::Value::Nil => {
                        return Err(mlua::Error::runtime(
                            "event.add() received nil as event tag",
                        ));
                    }

                    mlua::Value::UserData(ud) => ud,
                    other => {
                        return Err(mlua::Error::runtime(format!(
                            "event.add() expected an event tag, got {}",
                            other.type_name()
                        )));
                    }
                };

                // create custom tag type that will hold variant of command
                let tag = tag.borrow::<muxw_types::config::EventDiscriminant>()?;
                // NOTE: If you move that to the place where tag is required
                if !tag.is_ready() {
                    return Err(mlua::Error::runtime(format!(
                        "event tag {tag:?} requires argument(s) - call it first (e.g.: inactive(0.8))"
                    )));
                }

                let mut registry = lua
                    .app_data_mut::<EventRegistry>()
                    .ok_or_else(|| mlua::Error::runtime("Registry not initialized"))?;
                let id = registry.register(lua, *tag, callback)?;

                Ok(id)
            })
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.add function",
            })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.event.add function",
        })?;

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

#[derive(multi_index_map::MultiIndexMap, Debug)]
#[multi_index_derive(Debug)]
#[multi_index_hash(rustc_hash::FxBuildHasher)]
pub struct Event {
    #[multi_index(hashed_unique)]
    id: muxw_types::ids::Id,
    #[multi_index(hashed_non_unique)]
    discriminant: muxw_types::config::EventDiscriminant,

    registry_key: mlua::RegistryKey,
}

#[derive(Default)]
pub struct EventRegistry {
    id_source: muxw_types::ids::IdSource,
    events: MultiIndexEventMap,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            id_source: muxw_types::ids::IdSource::new(),
            events: MultiIndexEventMap::default(),
        }
    }

    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        discriminant: muxw_types::config::EventDiscriminant,
        callback: mlua::Function,
    ) -> mlua::Result<muxw_types::ids::Id> {
        let id = self.id_source.acquire();
        self.events.insert(Event {
            id,
            discriminant,
            registry_key: lua.create_registry_value(callback)?,
        });

        Ok(id)
    }

    pub fn fire(
        &self,
        lua: &mlua::Lua,
        command: &muxw_types::config::ConfigCommand,
    ) -> mlua::Result<()> {
        let events = self.events.get_by_discriminant(&command.discriminant());
        for event in events {
            let callback: mlua::Function = lua.registry_value(&event.registry_key)?;
            let context = lua.create_table().unwrap();
            command.create_context(&context)?;
            callback.call::<()>(context)?;
        }

        Ok(())
    }

    pub fn unregister(&mut self, lua: &mlua::Lua, id: muxw_types::ids::Id) {
        let event = self.events.remove_by_id(&id);
        // TODO: Check if that works nicely and also check if it's possible to get Null variant here
        if let Some(event) = event {
            lua.remove_registry_value(event.registry_key);
        }
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
