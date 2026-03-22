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

#[derive(Default)]
pub struct EventRegistry {
    next_id: EventId,
    entries: std::collections::HashMap<EventId, RegisteredEvent>,
}

#[derive(Debug)]
struct RegisteredEvent {
    tag: EventKind,
    callback: mlua::RegistryKey,
}

// TODO: After creating working demo move that to muxw_types crate
pub type EventId = u64;

#[derive(Debug, Clone)]
pub enum EventKind {
    Input(input::InputEventKind),
}

impl EventRegistry {
    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        tag: EventKind,
        callback: mlua::Function,
    ) -> mlua::Result<EventId> {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(
            id,
            RegisteredEvent {
                tag,
                callback: lua.create_registry_value(callback)?,
            },
        );

        Ok(id)
    }

    pub fn fire(&self, lua: &mlua::Lua, event: EventKind, timestamp: f64) -> mlua::Result<()> {
        for entry in self.entries.values() {
            if event.matches(&entry.tag) {
                let callback: mlua::Function = lua.registry_value(&entry.callback)?;
                let event_table = event.into_event_table(lua, timestamp)?;
                callback.call::<()>(event_table)?;
            }
        }

        Ok(())
    }

    pub fn unregister(&mut self, lua: &mlua::Lua, id: EventId) -> mlua::Result<()> {
        if let Some(entry) = self.entries.remove(&id) {
            lua.remove_registry_value(entry.callback)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl EventKind {
    pub fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (EventKind::Input(fired), EventKind::Input(tag)) => fired.matches(tag),
        }
    }

    pub fn into_event_table(&self, lua: &mlua::Lua, timestamp: f64) -> mlua::Result<mlua::Table> {
        let event = lua.create_table()?;
        event.set("timestamp", timestamp)?;
        match self {
            EventKind::Input(kind) => kind.populate_event_table(&event, lua)?,
        }

        Ok(event)
    }
}

impl mlua::UserData for EventKind {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {}

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::Call,
            |lua, this, args: mlua::MultiValue| {
                use input::InputEventKind;
                use input::keyboard::InputKeyboardEventKind;

                match this {
                    EventKind::Input(InputEventKind::Keyboard(
                        InputKeyboardEventKind::Inactivity { .. },
                    )) => {
                        let timeout_secs: u64 = args
                            .into_iter()
                            .next()
                            .ok_or_else(|| {
                                mlua::Error::runtime("Inactivity() requires timeout in seconds")
                            })
                            .and_then(|v| mlua::FromLua::from_lua(v, lua))?;

                        lua.create_userdata(EventKind::Input(InputEventKind::Keyboard(
                            InputKeyboardEventKind::Inactivity {
                                timeout_secs,
                                location: Default::default(),
                                keyboard: Default::default(),
                            },
                        )))
                    }
                    _ => Err(mlua::Error::runtime(
                        "This event kind is not parametrized and cannot be called",
                    )),
                }
            },
        );
    }
}

impl mlua::FromLua for EventKind {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => {
                let borrowed = ud.borrow::<EventKind>()?;
                Ok(borrowed.clone())
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "EventKind".into(),
                message: Some("expected EventKind userdata".into()),
            }),
        }
    }
}
