#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Device {}

// TODO: Later on remove the specific Key struct and use more broad thing (do the similar thing that
// happened to KeyboardCriteria (now DeviceLocation))
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Key {
    pub keycode: u32,
    pub keysym: u32,
    pub utf8: Option<String>,
}
