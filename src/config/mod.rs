mod tables;

pub struct Config {
    lua: mlua::Lua,
    state: std::rc::Rc<ConfigState>,
}

pub struct ConfigState {
    builder: std::rc::Rc<std::cell::RefCell<ConfigBuilder>>,
    snapshot: arc_swap::ArcSwap<ConfigSnapshot>,
}

pub struct ConfigBuilder {
    keyboards: Vec<tables::input::keyboard::KeyboardConfig>,
    mices: Vec<()>,
}

pub struct ConfigSnapshot {
    pub keyboards: std::sync::Arc<Vec<tables::input::keyboard::KeyboardConfig>>,
    pub mices: std::sync::Arc<Vec<()>>,
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

        let state = std::rc::Rc::new(ConfigState::new());

        lua.globals()
            .set(
                "Ray",
                tables::create_global_table(&lua, std::rc::Rc::clone(&state))?,
            )
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
        let builder = std::rc::Rc::new(std::cell::RefCell::new(ConfigBuilder {
            keyboards: Vec::new(),
            mices: Vec::new(),
        }));

        let snapshot = ConfigSnapshot {
            keyboards: std::sync::Arc::new(Vec::new()),
            mices: std::sync::Arc::new(Vec::new()),
        };

        Self {
            builder,
            snapshot: arc_swap::ArcSwap::from_pointee(snapshot),
        }
    }

    fn publish(&self) {
        let builder = self.builder.borrow();

        let snapshot = ConfigSnapshot {
            keyboards: std::sync::Arc::new(builder.keyboards.clone()),
            mices: std::sync::Arc::new(builder.mices.clone()),
        };

        self.snapshot.store(std::sync::Arc::new(snapshot));
    }

    pub fn snapshot(&self) -> std::sync::Arc<ConfigSnapshot> {
        self.snapshot.load_full()
    }
}

#[derive(Clone)]
pub struct ConfigStateHandle {
    publish: std::rc::Rc<dyn Fn()>,
}

impl ConfigStateHandle {
    pub fn new(state: std::rc::Rc<ConfigState>) -> Self {
        Self {
            publish: std::rc::Rc::new(move || state.publish()),
        }
    }

    pub fn publish(&self) {
        (self.publish)()
    }
}
