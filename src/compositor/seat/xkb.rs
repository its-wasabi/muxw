pub(super) struct XKB {
    context: xkbcommon::xkb::Context,
    compose: xkbcommon::xkb::compose::Table,

    keyboards: std::collections::HashMap<crate::backend::input::DeviceKey, Keyboard>,
}

struct Keyboard {
    state: xkbcommon::xkb::State,
    compose_state: xkbcommon::xkb::compose::State,

    keybamp: xkbcommon::xkb::Keymap,
}
