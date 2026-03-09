// TODO: Change name, seat, and port into single context type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardEvent {
    Added {
        name: String,
        seat: String,
        port: String,
    },

    Removed {
        name: String,
        seat: String,
        port: String,
    },

    Pressed {
        name: String,
        seat: String,
        port: String,

        keycode: u32,
        keysym: u32,
        utf8: Option<String>,
        context: (), // TODO: Make that contain keys pressed alongside
    },

    KeyRepeat {
        name: String,
        seat: String,
        port: String,

        keycode: u32,
        keysym: u32,
        utf8: Option<String>,
        context: (), // TODO: Make that contain keys pressed alongside
    },
}
