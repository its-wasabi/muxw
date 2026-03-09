use mlua::LuaSerdeExt;

fn table_to_comma_string(table: &mlua::Table) -> mlua::Result<String> {
    let items: Vec<String> = table
        .sequence_values()
        .collect::<Result<Vec<String>, _>>()?;
    Ok(items.join(","))
}

pub fn create_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_keyboard_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.input.keyboard table",
        })?;

    input_keyboard_table
        .set(
            "get",
            lua.create_function(|_, criteria: KeyboardCriteria| {
                Ok(KeyboardConfigContext::new(criteria))
            })
            .map_err(|_| crate::error::InitError::Mlua {
                action: "create Mux.input.keyboard.get() function",
            })?,
        )
        .map_err(|_| crate::error::InitError::Mlua {
            action: "set Mux.input.keyboard.get() function",
        })?;

    Ok(input_keyboard_table)
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Hash, Default)]
pub struct KeyboardCriteria {
    pub name: Option<String>,
    pub port: Option<String>,
    pub seat: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyboardConfig {
    pub layout: KeyboardLayout,
    pub options: KeyboardOptions,
}

#[derive(Debug, Clone, Default)]
pub struct KeyboardLayout(pub Option<String>);

#[derive(Debug, Clone, Default)]
pub struct KeyboardOptions(pub Option<String>);

#[derive(Clone, Default)]
pub struct KeyboardConfigContext {
    criteria: KeyboardCriteria,
    config: KeyboardConfig,
}

impl KeyboardConfigContext {
    fn new(criteria: KeyboardCriteria) -> Self {
        Self {
            criteria,
            config: KeyboardConfig::default(),
        }
    }
}

impl mlua::UserData for KeyboardConfigContext {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("layout", |lua, this, layout: KeyboardLayout| {
            this.config.layout = layout;
            crate::config::mutate_config(lua, |config| {
                config.keyboard_xkb.insert(
                    this.criteria.clone(),
                    KeyboardConfig {
                        layout: this.config.layout.clone(),
                        options: this.config.options.clone(),
                    },
                );
            });

            Ok(this.clone())
        });

        methods.add_method_mut("options", |lua, this, options: KeyboardOptions| {
            this.config.options = options;
            crate::config::mutate_config(lua, |config| {
                config.keyboard_xkb.insert(
                    this.criteria.clone(),
                    KeyboardConfig {
                        layout: this.config.layout.clone(),
                        options: this.config.options.clone(),
                    },
                );
            });

            Ok(this.clone())
        });
    }
}

impl KeyboardCriteria {
    pub fn from_input_device(input_device: &input::Device) -> Self {
        Self {
            name: Some(input_device.name().to_string()),
            port: Some(input_device.sysname().to_string()),
            seat: Some(input_device.seat().logical_name().to_string()),
        }
    }

    pub const fn is_wildcard(&self) -> bool {
        self.name.is_none() && self.port.is_none() && self.seat.is_none()
    }

    pub fn matches(&self, name: Option<&str>, port: Option<&str>, seat: Option<&str>) -> bool {
        if let Some(n) = &self.name
            && Some(n.as_str()) != name
        {
            return false;
        }

        if let Some(p) = &self.port
            && Some(p.as_str()) != port
        {
            return false;
        }

        if let Some(s) = &self.seat
            && Some(s.as_str()) != seat
        {
            return false;
        }

        true
    }
}

impl mlua::FromLua for KeyboardCriteria {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Nil => Ok(Self {
                name: None,
                port: None,
                seat: None,
            }),
            _ => lua.from_value(value),
        }
    }
}

impl mlua::FromLua for KeyboardLayout {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(Self(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(Self(Some(table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(Self(None)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Keyboard Layout"),
                message: Some(String::from("Expected String or table of Strings")),
            })?,
        }
    }
}

impl mlua::FromLua for KeyboardOptions {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(Self(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(Self(Some(table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(Self(None)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Keyboard Options"),
                message: Some(String::from("Expected String or table of Strings")),
            })?,
        }
    }
}
