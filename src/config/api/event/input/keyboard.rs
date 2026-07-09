// TODO: Use KeyboardCriteria type for passing name seat port

// use crate::config::api::input::keyboard::KeyboardConfigBuilder;

pub fn create_event_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let event_input_keyboard_table =
        lua.create_table()
            .map_err(|err| crate::error::InitError::Mlua {
                action: "create Mux.event.input.keyboard table",
            })?;

    // event_input_keyboard_table
    //     .set(
    //         "added",
    //         tag(
    //             InputKeyboardEventKind::Added {
    //                 keyboard: KeyboardConfigBuilder::default(),
    //                 location: muxw_types::input::DeviceLocation::default(),
    //             },
    //             "create Mux.event.input.keyboard.added tag",
    //         )?,
    //     )
    //     .map_err(|err| crate::error::InitError::Mlua {
    //         action: "set Mux.event.input.keyboard.added tag",
    //     })?;
    //
    // event_input_keyboard_table
    //     .set(
    //         "removed",
    //         tag(
    //             InputKeyboardEventKind::Removed {
    //                 location: muxw_types::input::DeviceLocation::default(),
    //             },
    //             "create Mux.event.input.keyboard.removed tag",
    //         )?,
    //     )
    //     .map_err(|err| crate::error::InitError::Mlua {
    //         action: "set Mux.event.input.keyboard.removed tag",
    //     })?;
    //
    // event_input_keyboard_table
    //     .set(
    //         "pressed",
    //         tag(
    //             InputKeyboardEventKind::Pressed {
    //                 location: muxw_types::input::DeviceLocation::default(),
    //                 key: muxw_types::input::keyboard::Key::default(),
    //             },
    //             "create Mux.event.input.keyboard.pressed tag",
    //         )?,
    //     )
    //     .map_err(|err| crate::error::InitError::Mlua {
    //         action: "set Mux.event.input.keyboard.pressed tag",
    //     })?;
    //
    // event_input_keyboard_table
    //     .set(
    //         "keyrepeat",
    //         tag(
    //             InputKeyboardEventKind::KeyRepeat {
    //                 location: muxw_types::input::DeviceLocation::default(),
    //                 key: muxw_types::input::keyboard::Key::default(),
    //             },
    //             "create Mux.event.input.keyboard.keyrepeat tag",
    //         )?,
    //     )
    //     .map_err(|err| crate::error::InitError::Mlua {
    //         action: "set Mux.event.input.keyboard.keyrepeat tag",
    //     })?;

    event_input_keyboard_table.set(
        "added",
        muxw_types::config::EventDiscriminant::KeyboardAdded,
    );

    event_input_keyboard_table.set(
        "inactive",
        muxw_types::config::EventDiscriminant::KeyboardInactive(None),
    );

    // event_input_keyboard_table
    //     .set(
    //         "inactivity",
    //         tag(
    //             InputKeyboardEventKind::Inactivity {
    //                 timeout_secs: None,
    //                 keyboard: KeyboardConfigBuilder::default(),
    //                 location: muxw_types::input::DeviceLocation::default(),
    //             },
    //             "create Mux.event.input.keyboard.inactivity tag",
    //         )?,
    //     )
    //     .map_err(|err| crate::error::InitError::Mlua {
    //         action: "set Mux.event.input.keyboard.inactivity tag",
    //     })?;
    //

    Ok(event_input_keyboard_table)
}

// #[derive(Debug, Clone)]
// pub enum InputKeyboardEventKind {
//     Added {
//         // IMPORTANT: That is right make it in the way that .get() function for keyboard returns
//         // keyboard builder with already linked found keyboard instead of just specifying criteria
//         // TODO: While working with callback of keyboard added you are still using the keyboard
//         // criteria (via KeyboardConfigBuilder), you should use only KeyboardConfig and apply
//         // directly to kb object stored or referenced by the passed event to fire
//         // 1. Swap KeyboardConfigBuilder to KeyboardConfig
//         // 2. Find a way to store data used by the event but not exposed to the user
//         // ! Remember to still update the Config struct in case of future updates
//         // IMPORTANT: Remember to do that, later it will be only harder to refactor
//         keyboard: crate::config::api::input::keyboard::KeyboardConfigBuilder,
//         location: muxw_types::input::DeviceLocation,
//     },
//     Removed {
//         location: muxw_types::input::DeviceLocation,
//     },
//
//     Pressed {
//         location: muxw_types::input::DeviceLocation,
//         key: muxw_types::input::keyboard::Key,
//     },
//     KeyRepeat {
//         location: muxw_types::input::DeviceLocation,
//         key: muxw_types::input::keyboard::Key,
//     },
//
//     Inactivity {
//         timeout_secs: Option<u64>,
//         keyboard: crate::config::api::input::keyboard::KeyboardConfigBuilder,
//         location: muxw_types::input::DeviceLocation,
//     },
// }
//
// impl super::super::ErasedEventKind for InputKeyboardEventKind {
//     fn matches(&self, other: &dyn super::super::ErasedEventKind) -> bool {
//         let Some(other) = other.as_any().downcast_ref::<Self>() else {
//             return false;
//         };
//
//         if std::mem::discriminant(self) != std::mem::discriminant(other) {
//             return false;
//         }
//
//         match (self, other) {
//             (
//                 Self::Inactivity {
//                     timeout_secs: self_timeout_secs,
//                     ..
//                 },
//                 Self::Inactivity {
//                     timeout_secs: other_timeout_secs,
//                     ..
//                 },
//             ) => self_timeout_secs == other_timeout_secs,
//             _ => true,
//         }
//     }
//
//     fn populate_event_table(&self, t: &mlua::Table, lua: &mlua::Lua) -> mlua::Result<()> {
//         match self {
//             Self::Added { keyboard, location } => {
//                 t.set("keyboard", keyboard.clone())?;
//                 t.set("location", location.clone())?;
//             }
//             Self::Removed { location } => {
//                 t.set("location", location.clone())?;
//             }
//             Self::Pressed { location, key } | Self::KeyRepeat { location, key } => {
//                 t.set("location", location.clone())?;
//                 t.set("key", key.clone())?;
//             }
//             Self::Inactivity {
//                 timeout_secs,
//                 keyboard,
//                 location,
//             } => {
//                 t.set("timeout_secs", *timeout_secs)?;
//                 t.set("keyboard", keyboard.clone())?;
//                 t.set("location", location.clone())?;
//             }
//         }
//         Ok(())
//     }
//
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
//
//     fn clone_box(&self) -> Box<dyn super::super::ErasedEventKind> {
//         Box::new(self.clone())
//     }
//
//     fn tag_name(&self) -> &'static str {
//         match self {
//             Self::Added { .. } => "added",
//             Self::Removed { .. } => "removed",
//             Self::Pressed { .. } => "pressed",
//             Self::KeyRepeat { .. } => "keyrepeat",
//             Self::Inactivity { .. } => "inactivity(secs)",
//         }
//     }
//
//     fn is_callable_ready(&self) -> bool {
//         match self {
//             Self::Inactivity { timeout_secs, .. } => timeout_secs.is_some(),
//             _ => true,
//         }
//     }
//
//     fn call_with_args(
//         &self,
//         lua: &mlua::Lua,
//         args: mlua::MultiValue,
//     ) -> mlua::Result<mlua::AnyUserData> {
//         match self {
//             Self::Inactivity { .. } => {
//                 let timeout_secs: u64 = args
//                     .into_iter()
//                     .next()
//                     .ok_or_else(|| {
//                         mlua::Error::runtime("inactivity() requires a timeout in seconds")
//                     })
//                     .and_then(|v| mlua::FromLua::from_lua(v, lua))?;
//
//                 lua.create_userdata(super::super::EventTag(Box::new(Self::Inactivity {
//                     timeout_secs: Some(timeout_secs),
//                     keyboard: KeyboardConfigBuilder::default(),
//                     location: muxw_types::input::DeviceLocation::default(),
//                 })))
//             }
//             _ => Err(mlua::Error::runtime("This event kind is not parameterized")),
//         }
//     }
// }
