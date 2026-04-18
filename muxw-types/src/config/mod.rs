/// When other threads require something from the config

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigCommand {
    Exit,
    Reload,
    // TODO: Make that store actual keyboard object
    KeyboardAdded,
    KeyboardInactive(std::time::Duration),
}

/// When config requires other threads to do something
pub trait ConfigEvent: Send + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventDiscriminant {
    /// Runs on compositor exit (last chance to run some lua)
    Exit,
    /// Only indication to notify config about fact that it was reloaded
    Reload,
    InputKeyboardAdded,
    InputKeyboardRemoved,
    InputKeyboardPressed,
    InputKeyboardInactive(Option<std::time::Duration>),
}

// TODO: Make trait that can be applied on config_command and return data needed by this system
pub trait ConfigApiEvent: Send + 'static {
    fn discriminant(&self) -> EventDiscriminant;
}

impl ConfigApiEvent for ConfigCommand {
    fn discriminant(&self) -> EventDiscriminant {
        match self {
            Self::Exit => EventDiscriminant::Exit,
            Self::Reload => EventDiscriminant::Reload,
            Self::KeyboardAdded => EventDiscriminant::InputKeyboardAdded,
            Self::KeyboardInactive(duration) => {
                EventDiscriminant::InputKeyboardInactive(Some(*duration))
            }
        }
    }
}

impl EventDiscriminant {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        match self {
            Self::InputKeyboardInactive(None) => false,
            _ => true,
        }
    }
}

impl mlua::UserData for EventDiscriminant {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::Call,
            |lua, this, args: mlua::MultiValue| match this {
                Self::InputKeyboardInactive(..) => {
                    let duration: f64 = args
                        .into_iter()
                        .next()
                        .ok_or_else(|| {
                            mlua::Error::runtime(
                                "inactive(sec) requires time in seconds (accepts floats)",
                            )
                        })
                        .and_then(|v| mlua::FromLua::from_lua(v, lua))?;
                    let duration = (duration * 1000.0) as u64;
                    lua.create_userdata(Self::InputKeyboardInactive(Some(
                        std::time::Duration::from_millis(duration),
                    )))
                }

                other => Err(mlua::Error::runtime(format!(
                    "event tag {other:?} is not callable"
                ))),
            },
        );
    }
}

impl ConfigCommand {
    pub fn create_context(self, _context: &mlua::Table) {
        match self {
            Self::Exit => (),
            Self::Reload => todo!("Give it some reload info"),
            // ConfigCommand::KeyboardAdded => todo!("give it keyboard obj"),
            // ConfigCommand::KeyboardInactive(_) => todo!("give it keyboard obj"),
            _ => println!("IMPORTANT: Unimplemented code (this line is urgent to be removed)"),
        }
    }
}
