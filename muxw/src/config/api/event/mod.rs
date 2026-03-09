// TODO: Merge set.rs and mod.rs into single file
pub mod keyboard;
pub mod set;

use mlua::LuaSerdeExt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Event {
    kind: EventKind,
    timestamp: std::time::Instant,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Keyboard(keyboard::KeyboardEvent),
}

pub fn create_event_table(lua: &mlua::Lua) -> Result<mlua::Table, crate::error::InitError> {
    let event_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.event table",
        })?;

    let event_manager = EventManager::default();

    event_table.set(
        "set",
        set::create_event_set_function(lua).map_err(|_| crate::error::InitError::Mlua {
            action: "set Mux.event.set() function",
        })?,
    );

    Ok(event_table)
}

impl mlua::UserData for EventType {}

// TODO: Move it to separate files for different devices and make it Keyboard(KbEbentType) here
enum EventTypeCtx {
    KeyboardAdded { name: String, seat: String },
}

fn ev(lua: &mlua::Lua, t: EventType) -> mlua::Result<mlua::AnyUserData> {
    lua.create_userdata(t)
}

pub fn create_event_type_subtables(lua: &mlua::Lua, event_table: &mlua::Table) -> mlua::Result<()> {
    let keyboard = lua.create_table()?;
    keyboard.set("added", ev(lua, EventType::KeyboardAdded)?)?;
    keyboard.set("removed", ev(lua, EventType::KeyboardRemoved)?)?;
    event_table.set("keyboard", keyboard)?;

    let mouse = lua.create_table()?;
    mouse.set("added", ev(lua, EventType::MouseAdded)?)?;
    mouse.set("removed", ev(lua, EventType::MouseRemoved)?)?;
    event_table.set("mouse", mouse)?;

    let monitor = lua.create_table()?;
    monitor.set("added", ev(lua, EventType::MonitorAdded)?)?;
    monitor.set("removed", ev(lua, EventType::MonitorRemoved)?)?;
    event_table.set("monitor", monitor)?;

    let window = lua.create_table()?;
    let window_focus = lua.create_table()?;
    window_focus.set("gain", ev(lua, EventType::WindowFocusGain)?)?;
    window_focus.set("lose", ev(lua, EventType::WindowFocusLose)?)?;
    window.set("focus", window_focus)?;
    event_table.set("window", window)?;

    let workspace = lua.create_table()?;
    let workspace_focus = lua.create_table()?;
    workspace_focus.set("gain", ev(lua, EventType::WorkspaceFocusGain)?)?;
    workspace_focus.set("lose", ev(lua, EventType::WorkspaceFocusLose)?)?;
    workspace.set("focus", workspace_focus)?;
    event_table.set("workspace", workspace)?;

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Hash)]
pub enum EventType {
    KeyboardAdded,
    KeyboardRemoved,
    MouseAdded,
    MouseRemoved,
    MonitorAdded,
    MonitorRemoved,
    WindowFocusGain,
    WindowFocusLose,
    WorkspaceFocusGain,
    WorkspaceFocusLose,
}

impl mlua::FromLua for EventType {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        lua.from_value(value)
    }
}

#[derive(Default)]
pub struct EventManager {
    registry: std::collections::HashMap<Event, Vec<mlua::RegistryKey>>,
}

impl EventManager {
    pub fn register(
        &mut self,
        lua: &mlua::Lua,
        event: Event,
        callback: mlua::Function,
    ) -> mlua::Result<()> {
        let key = lua.create_registry_value(callback)?;
        self.registry.entry(event).or_default().push(key);
        Ok(())
    }
    pub fn call(&self, lua: &mlua::Lua, event: &Event) -> mlua::Result<()> {
        if let Some(keys) = self.registry.get(&event) {
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
