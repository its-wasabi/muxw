pub fn create_input_keyboard_table(
    lua: &mlua::Lua,
) -> Result<mlua::Table, crate::error::InitError> {
    let input_keyboard_table = lua
        .create_table()
        .map_err(|err| crate::error::InitError::Mlua {
            action: "create Mux.input.keyboard table",
        })?;

    input_keyboard_table
        .set(
            "get",
            lua.create_function(|_, criteria: muxw_types::input::Criteria| {
                Ok(KeyboardConfigBuilder {
                    criteria,
                    config: muxw_types::input::keyboard::Config::default(),
                })
            })
            .map_err(|_| crate::error::InitError::Mlua {
                action: "create Mux.input.keyboard.get() function",
            })?,
        )
        .map_err(|_| crate::error::InitError::Mlua {
            action: "set Mux.input.keyboard.get() function",
        })?;

    Ok(input_keyboard_table)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct KeyboardConfigBuilder {
    criteria: muxw_types::input::Criteria,
    config: muxw_types::input::keyboard::Config,
}

impl mlua::UserData for KeyboardConfigBuilder {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "layout",
            |lua, this, layout: muxw_types::input::keyboard::XkbLayout| {
                this.config.layout = layout;
                crate::config::mutate_config(lua, |config| {
                    config
                        .keyboard_xkb
                        .insert(this.criteria.clone(), this.config.clone());
                })
                .unwrap();

                Ok(this.to_owned())
            },
        );

        methods.add_method_mut(
            "options",
            |lua, this, options: muxw_types::input::keyboard::XkbOptions| {
                this.config.options = options;
                crate::config::mutate_config(lua, |config| {
                    config
                        .keyboard_xkb
                        .insert(this.criteria.clone(), this.config.clone());
                });

                Ok(this.to_owned())
            },
        );
    }
}
