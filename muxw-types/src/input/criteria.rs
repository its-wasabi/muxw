#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DeviceLocation {
    pub name: Option<String>,
    pub port: Option<String>,
    pub seat: Option<String>,
}

impl DeviceLocation {
    #[must_use = "creates new instance of Criteria"]
    pub fn from_input_device(input_device: &input::Device) -> Self {
        Self {
            name: Some(input_device.name().to_string()),
            port: Some(input_device.sysname().to_string()),
            seat: Some(input_device.seat().logical_name().to_string()),
        }
    }

    #[must_use]
    pub const fn is_wildcard(&self) -> bool {
        self.name.is_none() && self.port.is_none() && self.seat.is_none()
    }

    #[must_use]
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

impl mlua::FromLua for DeviceLocation {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        if value == mlua::Value::Nil {
            Ok(Self {
                name: None,
                port: None,
                seat: None,
            })
        } else {
            use mlua::LuaSerdeExt;
            lua.from_value(value)
        }
    }
}
