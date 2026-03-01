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
            source: err,
        })?;

    input_keyboard_table
        .set(
            "get",
            lua.create_function(move |_, criteria: KeyboardCriteria| {
                Ok(KeyboardConfigBuilder::new(criteria))
            })
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.input.keyboard.get() function",
                source: err,
            })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Mux.input.keyboard.get() function",
            source: err,
        })?;

    Ok(input_keyboard_table)
}

#[derive(Debug, Clone)]
pub struct KeyboardConfig {
    pub layout: KeyboardLayout,
    pub options: KeyboardOptions,
}

#[derive(Clone)]
pub struct KeyboardConfigBuilder {
    criteria: KeyboardCriteria,
    layout: KeyboardLayout,
    options: KeyboardOptions,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Hash)]
pub struct KeyboardCriteria {
    pub name: Option<String>,
    pub port: Option<String>,
    pub seat: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeyboardLayout(pub Option<String>);

#[derive(Debug, Clone)]
pub struct KeyboardOptions(pub Option<String>);

impl KeyboardConfigBuilder {
    const fn new(criteria: KeyboardCriteria) -> Self {
        Self {
            criteria,
            layout: KeyboardLayout(None),
            options: KeyboardOptions(None),
        }
    }
}

impl From<KeyboardConfigBuilder> for KeyboardConfig {
    fn from(value: KeyboardConfigBuilder) -> Self {
        Self {
            layout: value.layout,
            options: value.options,
        }
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

    pub fn matches(
        &self,
        kb_name: Option<&str>,
        kb_port: Option<&str>,
        kb_seat: Option<&str>,
    ) -> bool {
        #[allow(clippy::collapsible_if)]
        if let Some(name) = &self.name {
            if Some(name.as_ref()) != kb_name {
                return false;
            }
        }
        #[allow(clippy::collapsible_if)]
        if let Some(port) = &self.port {
            if Some(port.as_ref()) != kb_port {
                return false;
            }
        }
        #[allow(clippy::collapsible_if)]
        if let Some(seat) = &self.seat {
            if Some(seat.as_ref()) != kb_seat {
                return false;
            }
        }

        true
    }
}

impl mlua::UserData for KeyboardConfigBuilder {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("layout", |_, this, layout: KeyboardLayout| {
            this.layout = layout;
            Ok(this.clone())
        });

        methods.add_method_mut("options", |_, this, options: KeyboardOptions| {
            this.options = options;
            Ok(this.clone())
        });

        methods.add_method("apply", |_, this, ()| {
            crate::config::CONFIG
                .keyboard
                .insert(this.criteria.clone(), KeyboardConfig::from(this.clone()));
            Ok(())
        });
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
            }),
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
            }),
        }
    }
}
