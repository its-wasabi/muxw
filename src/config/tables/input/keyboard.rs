use mlua::LuaSerdeExt;

pub fn create_input_keyboard_table(
    lua: &mlua::Lua,
    state: std::rc::Rc<crate::config::ConfigState>,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_keyboard_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Muxw.input.keyboard table",
            source: err,
        })?;

    input_keyboard_table
        .set(
            "get",
            lua.create_function(move |_, criteria: KeyboardCriteria| {
                Ok(KeyboardConfigBuilder::new(
                    criteria,
                    std::rc::Rc::clone(&state),
                ))
            })
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Muxw.input.keyboard.get() function",
                source: err,
            })?,
        )
        .map_err(|err| crate::error::InitError::Mlua {
            action: "set Muxw.input.keyboard.get() function",
            source: err,
        })?;

    Ok(input_keyboard_table)
}

fn table_to_comma_string(table: &mlua::Table) -> mlua::Result<String> {
    let items: Vec<String> = table
        .sequence_values()
        .collect::<Result<Vec<String>, _>>()?;
    Ok(items.join(","))
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

    state: std::rc::Rc<crate::config::ConfigState>,
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
    fn new(criteria: KeyboardCriteria, state: std::rc::Rc<crate::config::ConfigState>) -> Self {
        Self {
            criteria,
            layout: KeyboardLayout(None),
            options: KeyboardOptions(None),
            state,
        }
    }
}

impl crate::config::Publishable for KeyboardConfigBuilder {
    fn apply(&self, state: &crate::config::ConfigState) {
        here!(
            "CONFIG: Apply {:?} options: {:?}, layout: {:?}",
            self.criteria,
            self.options,
            self.layout
        );

        state.with_builder(|builder| {
            builder.keyboards.insert(
                self.criteria.clone(),
                KeyboardConfig {
                    layout: self.layout.clone(),
                    options: self.options.clone(),
                },
            );
        });

        state.publish();
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

    pub fn is_wildcard(&self) -> bool {
        self.name.is_none() && self.port.is_none() && self.seat.is_none()
    }

    // TODO: Make that foo exit early if doesn't match
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
            Ok(())
        });

        methods.add_method_mut("options", |_, this, options: KeyboardOptions| {
            this.options = options;
            Ok(())
        });

        methods.add_method("apply", |_, this, ()| {
            use crate::config::Publishable;
            this.apply(&this.state);

            Ok(())
        });
    }
}

impl mlua::FromLua for KeyboardCriteria {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Nil => Ok(KeyboardCriteria {
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
            mlua::Value::String(string) => Ok(KeyboardLayout(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(KeyboardLayout(Some(table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(KeyboardLayout(None)),
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
            mlua::Value::String(string) => Ok(KeyboardOptions(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(KeyboardOptions(Some(table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(KeyboardOptions(None)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Keyboard Options"),
                message: Some(String::from("Expected String or table of Strings")),
            }),
        }
    }
}
