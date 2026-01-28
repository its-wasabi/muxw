use mlua::LuaSerdeExt;

pub fn create_input_keyboard_table(
    lua: &mlua::Lua,
    state: std::rc::Rc<crate::config::ConfigState>,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_keyboard_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Ray.input.keyboard table",
            source: err,
        })?;

    let builder = state.builder.clone();
    let handle = crate::config::ConfigStateHandle::new(state);

    input_keyboard_table.set(
        "get",
        lua.create_function(move |_, criteria: KeyboardCriteria| {
            Ok(KeyboardConfigBuilder::new(
                criteria,
                builder.clone(),
                handle.clone(),
            ))
        })
        .unwrap(),
    );

    Ok(input_keyboard_table)
}

fn table_to_comma_string(table: &mlua::Table) -> mlua::Result<String> {
    let items: Vec<String> = table
        .sequence_values()
        .collect::<Result<Vec<String>, _>>()?;
    Ok(items.join(","))
}

#[derive(Clone)]
pub struct KeyboardConfig {
    pub criteria: KeyboardCriteria,
    pub layout: Option<KeyboardLayout>,
    pub options: Option<KeyboardOptions>,
}

#[derive(Clone)]
pub struct KeyboardConfigBuilder {
    criteria: KeyboardCriteria,
    layout: Option<KeyboardLayout>,
    options: Option<KeyboardOptions>,

    builder: std::rc::Rc<std::cell::RefCell<crate::config::ConfigBuilder>>,
    state: crate::config::ConfigStateHandle,
}

#[derive(Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct KeyboardCriteria {
    pub name: Option<String>,
    pub port: Option<String>,
    pub seat: Option<String>,
}

#[derive(Clone)]
pub struct KeyboardLayout(pub String);

#[derive(Clone)]
pub struct KeyboardOptions(pub String);

impl KeyboardConfigBuilder {
    fn new(
        criteria: KeyboardCriteria,
        builder: std::rc::Rc<std::cell::RefCell<crate::config::ConfigBuilder>>,
        state: crate::config::ConfigStateHandle,
    ) -> Self {
        Self {
            criteria,
            layout: None,
            options: None,
            builder,
            state,
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
        let name_matches = self
            .name
            .as_ref()
            .is_none_or(|n| Some(n.as_str()) == kb_name);
        let port_matches = self
            .port
            .as_ref()
            .is_none_or(|p| Some(p.as_str()) == kb_port);
        let seat_matches = self
            .seat
            .as_ref()
            .is_none_or(|s| Some(s.as_str()) == kb_seat);

        name_matches && port_matches && seat_matches
    }
}

impl mlua::UserData for KeyboardConfigBuilder {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("layout", |_, this, layout: KeyboardLayout| {
            this.layout = Some(layout);
            Ok(this.clone())
        });

        methods.add_method_mut("options", |_, this, options: KeyboardOptions| {
            this.options = Some(options);
            Ok(this.clone())
        });

        methods.add_method("apply", |_, this, ()| {
            {
                let mut builder = this.builder.borrow_mut();

                builder.keyboards.push(KeyboardConfig {
                    criteria: this.criteria.clone(),
                    layout: this.layout.clone(),
                    options: this.options.clone(),
                });
            }

            this.state.publish();

            Ok(())
        });
    }
}

impl mlua::FromLua for KeyboardCriteria {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        lua.from_value(value)
    }
}

impl mlua::FromLua for KeyboardLayout {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(KeyboardLayout(string.to_str()?.to_string())),
            mlua::Value::Table(table) => Ok(KeyboardLayout(table_to_comma_string(&table)?)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Layout"),
                message: Some(String::from("Expected String or table of Strings")),
            }),
        }
    }
}

impl mlua::FromLua for KeyboardOptions {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(KeyboardOptions(string.to_str()?.to_string())),
            mlua::Value::Table(table) => Ok(KeyboardOptions(table_to_comma_string(&table)?)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Optins"),
                message: Some(String::from("Expected String or table of Strings")),
            }),
        }
    }
}
