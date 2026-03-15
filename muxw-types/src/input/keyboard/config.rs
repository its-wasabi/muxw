#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub layout: XkbLayout,
    pub options: XkbOptions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct XkbLayout(pub Option<String>);

impl mlua::FromLua for XkbLayout {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(Self(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(Self(Some(super::table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(Self(None)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Keyboard Layout"),
                message: Some(String::from("Expected String or table of Strings")),
            })?,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct XkbOptions(pub Option<String>);

impl mlua::FromLua for XkbOptions {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(Self(Some(string.to_str()?.to_string()))),
            mlua::Value::Table(table) => Ok(Self(Some(super::table_to_comma_string(&table)?))),
            mlua::Value::Nil => Ok(Self(None)),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("Keyboard Options"),
                message: Some(String::from("Expected String or table of Strings")),
            })?,
        }
    }
}
