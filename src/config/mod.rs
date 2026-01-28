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
    pub fn new(path: &std::path::Path) -> Result<Self, crate::error::InitError> {
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

        let config_source = Self::read_config_source(path).unwrap(); //TODO: .map_err(|err|)?;
        lua.load(config_source)
            .exec()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "load & exec config",
                source: err,
            })?;

        Ok(Self { lua, state })
    }

    fn read_config_source(path: &std::path::Path) -> std::io::Result<String> {
        match std::fs::read_to_string(path) {
            Ok(source) => Ok(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::create_default_config(path)?;
                std::fs::read_to_string(path)
            }
            Err(err) => Err(err),
        }
    }

    pub fn create_default_config(path: &std::path::Path) -> std::io::Result<()> {
        here!("Config doesn't exist - creating default");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, crate::DEFAULT_CONFIG)
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
