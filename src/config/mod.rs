mod tables;

#[derive(Debug)]
pub struct Config {
    lua: mlua::Lua,
    pub state: ConfigState,
}

// Keep here config structs only for the specyfik part of program so you can clone smaller part
// of the config state and pass that to the part of program that requires it
// And thanks to that you can store jus
#[derive(Debug)]
pub struct ConfigState {
    keyboard: Vec<tables::input::keyboard::KeyboardConfig>,
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

        let state = std::rc::Rc::new(std::cell::RefCell::new(ConfigState::new()));

        lua.globals()
            .set(
                "Ray",
                tables::create_global_table(&lua, std::rc::Rc::clone(&state))?,
            )
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set global Ray",
                source: err,
            });

        let state = std::rc::Rc::try_unwrap(state)
            .expect("HANDLE ERRORS - Failed to unwrap")
            .into_inner();

        Ok(Self { lua, state })
    }
}

impl ConfigState {
    fn new() -> Self {
        Self {
            keyboard: Vec::new(),
        }
    }
}
