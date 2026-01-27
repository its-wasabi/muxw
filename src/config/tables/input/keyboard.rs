pub fn create_input_keyboard_table(
    lua: &mlua::Lua,
    state: std::rc::Rc<std::cell::RefCell<crate::config::ConfigState>>,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_keyboard_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Ray.input.keyboard table",
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
    pub criteria: KeyboardCriteria,
    pub layout: Option<KeyboardLayout>,
    pub options: Option<KeyboardOptions>,

    config_state: std::rc::Weak<std::cell::RefCell<crate::config::ConfigState>>,
}

#[derive(Debug, Clone)]
pub struct KeyboardCriteria {
    pub name: Option<String>,
    pub port: Option<String>,
    pub seat: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeyboardLayout(pub String);

#[derive(Debug, Clone)]
pub struct KeyboardOptions(pub String);

impl KeyboardConfig {
    fn new(
        criteria: KeyboardCriteria,
        state: std::rc::Weak<std::cell::RefCell<crate::config::ConfigState>>,
    ) -> Self {
        Self {
            criteria,
            config_state: state,
            layout: None,
            options: None,
        }
    }
}

impl mlua::UserData for KeyboardConfig {
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
            here!(
                "Apply called layout: {:?}, options: {:?}",
                this.layout,
                this.options
            );

            if let Some(state) = this.config_state.upgrade() {
                state.borrow_mut().keyboard.push(this.clone());
            }

            Ok(())
        });
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
