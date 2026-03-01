use std::collections::HashMap;

mod api;

pub static CONFIG: muxw_types::Global<Config> = muxw_types::Global::new();

pub struct ConfigContext {
    lua: mlua::Lua,
}

impl ConfigContext {
    pub fn new(path: &std::path::Path) -> Result<Self, crate::error::InitError> {
        CONFIG.init(Config::default());

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

        let mux_table = api::create_global_table(&lua)?;
        lua.globals()
            .set("Mux", mux_table)
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set Mux table",
                source: err,
            });

        let config_source =
            Self::read_config_source(path).map_err(|err| crate::error::InitError::Io {
                action: "read config file",
                path: Some(path.into()),
                source: err,
            })?;
        #[allow(clippy::expect_used)]
        lua.load(config_source)
            .exec()
            .expect("Handle errors diferently for running code");

        here!("Config: {CONFIG:#?}");

        Ok(Self { lua })
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

#[derive(Debug, Default)]
pub struct Config {
    keyboard: muxw_types::ConfigField<
        HashMap<api::input::keyboard::KeyboardCriteria, api::input::keyboard::KeyboardConfig>,
    >,
}
