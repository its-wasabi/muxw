mod tables;

pub struct Config {
    lua: mlua::Lua,
    shared: SharedConfig,
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

        let share = SharedConfig {
            keyboards: crate::utils::types::config_cell::ConfigSection::new(),
        };

        lua.globals()
            .set(
                "Mux",
                tables::create_global_table(&lua, std::rc::Rc::clone(&state))?,
            )
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set global Mux",
                source: err,
            });

        let config_source = Self::read_config_source(path).unwrap(); // TODO: .map_err(|err|)?;
        lua.load(config_source)
            .exec()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "load & exec config",
                source: err,
            })?;

        here!("CONFIG STATE BUILDER: {:#?}", state.builder);

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

pub struct SharedConfig {
    pub keyboards: crate::utils::types::config_cell::ConfigSection<
        std::collections::HashMap<
            tables::input::keyboard::KeyboardCriteria,
            tables::input::keyboard::KeyboardConfig,
        >,
    >,
}

impl SharedConfig {
    fn new() -> Self {
        Self {
            keyboards: crate::utils::types::config_cell::ConfigSection::new(),
        }
    }
}
