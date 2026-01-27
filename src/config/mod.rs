mod tables;

#[derive(Debug)]
pub struct Config {
    lua: mlua::Lua,
    pub state: ConfigState,
}

#[derive(Debug)]
pub struct ConfigState {
    pub keyboard: std::sync::Arc<parking_lot::RwLock<Vec<tables::input::keyboard::KeyboardConfig>>>,
    pub mouse: std::sync::Arc<parking_lot::RwLock<Vec<()>>>,
}

impl Config {
    pub fn new() -> Result<Self, crate::error::InitError> {
        let libs = mlua::StdLib::TABLE
            | mlua::StdLib::MATH
            | mlua::StdLib::STRING
            | mlua::StdLib::IO
            | mlua::StdLib::OS;
        let options = mlua::LuaOptions::default();
        let lua =
            mlua::Lua::new_with(libs, options).map_err(|err| crate::error::InitError::Mlua {
                action: "init lua",
                source: err,
            })?;

        let state = ConfigState::new();

        lua.globals()
            .set("Ray", tables::create_global_table(&lua, &state)?)
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set global Ray",
                source: err,
            });

        // EXEC HERE

        Ok(Self { lua, state })
    }
}

impl ConfigState {
    fn new() -> Self {
        Self {
            keyboard: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
            mouse: std::sync::Arc::new(parking_lot::RwLock::new(Vec::new())),
        }
    }
}
