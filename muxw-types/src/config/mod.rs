// TODO: Move all mlua related code out of muxw_types into muxw crate

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigCommand {
    Exit,
    Reload,
    // TODO: Make that store actual keyboard object
    KeyboardAdded { device: () },
    // FIX: Passing duration in a command feels like wrong move here (well systems wont listen and
    // set inactivity notify to config thread every 0.0000000000001 second to make sure that none
    // inactivity event is missed) try making some system that assigning inactivity event registers
    // inactivity listener in related input manager, and make it return maybe some way to reference
    // that inactivity callback (but not lua function that only config thread can touch) because
    // there might be some problems with precision i think (but I'm not sure)
    // TODO: Change that to id instead of duration
    KeyboardInactive(std::time::Duration),
}

pub enum ConfigEvent {
    // NOTE: I somehow dislike that approach think about something else you can do
    RegisterKeyboardInactivityListener { duration: std::time::Duration },
    // TODO: Change that to id instead of duration
    UnregisterKeyboardInactivityListener { duration: std::time::Duration },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventDiscriminant {
    /// Runs on compositor exit (last chance to run some lua)
    Exit,
    /// Only indication to notify config about fact that it was reloaded
    Reload,
    KeyboardAdded,
    KeyboardRemoved,
    KeyboardPressed,
    KeyboardInactive(Option<std::time::Duration>),
    MouseInactive(Option<std::time::Duration>),
}

impl ConfigCommand {
    #[must_use]
    pub const fn discriminant(&self) -> EventDiscriminant {
        match self {
            Self::Exit => EventDiscriminant::Exit,
            Self::Reload => EventDiscriminant::Reload,
            Self::KeyboardAdded { .. } => EventDiscriminant::KeyboardAdded,
            Self::KeyboardInactive(duration) => {
                EventDiscriminant::KeyboardInactive(Some(*duration))
            }
        }
    }
}

impl EventDiscriminant {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        !matches!(
            self,
            Self::KeyboardInactive(None) | Self::MouseInactive(None)
        )
    }
}

impl mlua::UserData for EventDiscriminant {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::Call,
            |lua, this, args: mlua::MultiValue| match this {
                Self::KeyboardInactive(..) => {
                    let duration: f64 = args
                        .into_iter()
                        .next()
                        .ok_or_else(|| {
                            mlua::Error::runtime(
                                "inactive(sec) requires time in seconds (accepts floats)",
                            )
                        })
                        .and_then(|v| mlua::FromLua::from_lua(v, lua))?;
                    lua.create_userdata(Self::KeyboardInactive(Some(
                        std::time::Duration::from_secs_f64(duration),
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
    pub fn create_context(self, _context: &mlua::Table) -> mlua::Result<()> {
        match self {
            // Self::Exit => (),
            // Self::Reload => todo!("Give it some reload info"),
            // ConfigCommand::KeyboardAdded => todo!("give it keyboard obj"),
            // ConfigCommand::KeyboardInactive(_) => todo!("give it keyboard obj"),
            other => println!(
                "IMPORTANT({other:?}): Unimplemented code (this line is urgent to be removed)"
            ),
        }

        Ok(())
    }
}
