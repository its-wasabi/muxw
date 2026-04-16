pub mod api;

pub struct Config {
    lua: mlua::Lua,
    path: std::path::PathBuf,

    command: std::sync::mpsc::Receiver<CommandMessage>,
    event: std::sync::mpsc::SyncSender<Box<dyn muxw_types::config::ConfigEvent>>,
}

// IMPORTANT: Try to figure out some other way
/// # Unsafe
/// Send is required on struct with non send fields because struct need to be hand over to the
/// config thread once created, it is safe because after initialization no operation is done on the
/// struct data until config loop is ran already inside thread
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for Config {}

pub struct PendingCommand(std::sync::mpsc::Receiver<Result<(), crate::error::InitError>>);

impl PendingCommand {
    pub fn wait(self) -> Result<(), crate::error::InitError> {
        self.0.recv().map_err(|_| crate::error::InitError::Io {
            action: "config thread died before completing command",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })?
    }
}

type CommandMessage = (
    muxw_types::config::ConfigCommand,
    std::sync::mpsc::SyncSender<Result<(), crate::error::InitError>>,
);

pub struct CommandHandle(std::sync::mpsc::SyncSender<CommandMessage>);

impl CommandHandle {
    pub fn send(
        &self,
        command: muxw_types::config::ConfigCommand,
    ) -> Result<PendingCommand, crate::error::InitError> {
        let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
        self.0
            .send((command, done_tx))
            .map_err(|error| crate::error::InitError::Io {
                action: "send command to config thread",
                path: None,
                source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            })?;

        Ok(PendingCommand(done_rx))
    }
}

impl Drop for CommandHandle {
    fn drop(&mut self) {
        self.send(muxw_types::config::ConfigCommand::Exit);
    }
}

impl Config {
    pub fn spawn(
        path: &std::path::Path,
    ) -> Result<
        (
            std::rc::Rc<CommandHandle>,
            std::sync::mpsc::Receiver<Box<dyn muxw_types::config::ConfigEvent>>,
        ),
        crate::error::InitError,
    > {
        let (command_tx, command_rx) = std::sync::mpsc::sync_channel::<CommandMessage>(10);
        let command_handle = CommandHandle(command_tx);
        let (event_tx, event_rx) =
            std::sync::mpsc::sync_channel::<Box<dyn muxw_types::config::ConfigEvent>>(10);

        let _ = std::thread::Builder::new()
            .name("config".into())
            .spawn({
                let config = Self::new(path, command_rx, event_tx)?;

                move || config.run()
            })
            .map_err(|_| crate::error::InitError::Io {
                action: "create thread",
                path: None,
                source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            })?;

        Ok((std::rc::Rc::new(command_handle), event_rx))
    }

    fn new(
        path: &std::path::Path,
        command: std::sync::mpsc::Receiver<CommandMessage>,
        event: std::sync::mpsc::SyncSender<Box<dyn muxw_types::config::ConfigEvent>>,
    ) -> Result<Self, crate::error::InitError> {
        let lua = Self::load_config(path)?;
        let path = path.to_owned();

        let mut config = Self {
            lua,
            path,
            command,
            event,
        };

        Ok(config)
    }

    fn load_config(path: &std::path::Path) -> Result<mlua::Lua, crate::error::InitError> {
        let libs = mlua::StdLib::TABLE
            | mlua::StdLib::MATH
            | mlua::StdLib::STRING
            | mlua::StdLib::IO
            | mlua::StdLib::OS;
        let options = mlua::LuaOptions::default();

        let lua = mlua::Lua::new_with(libs, options)
            .map_err(|err| crate::error::InitError::Mlua { action: "init lua" })?;

        lua.set_app_data(api::event::EventRegistry::default());

        let mux_table = api::create_global_table(&lua)?;
        lua.globals()
            .set("Mux", mux_table)
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set Mux table",
            })?;

        let source = Self::read_config_source(path).map_err(|err| crate::error::InitError::Io {
            action: "read config file",
            path: Some(path.to_owned()),
            source: err,
        })?;

        // TODO: Unwrap is only temporally until there will be mechanism for handling lua errors
        lua.load(&source).exec().unwrap();
        // .map_err(|e| crate::error::InitError::Mlua {
        //     action: "execute config file",
        // })?;

        Ok(lua)
    }

    fn read_config_source(path: &std::path::Path) -> std::io::Result<String> {
        match std::fs::read_to_string(path) {
            Ok(source) => Ok(source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Self::create_default_config(path)?;
                std::fs::read_to_string(path)
            }
            Err(err) => Err(err),
        }
    }

    pub fn create_default_config(path: &std::path::Path) -> std::io::Result<()> {
        #[cfg(debug_assertions)]
        here!("Config doesn't exist - creating default");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, crate::DEFAULT_CONFIG)
    }

    fn run(mut self) -> Result<(), crate::error::InitError> {
        loop {
            let (command, done_tx) = self
                .command
                .recv()
                .map_err(|err| crate::error::InitError::Mlua { action: "todo" })?;

            match command {
                muxw_types::config::ConfigCommand::Exit => {
                    here!("EXIT");
                    return Ok(());
                }

                muxw_types::config::ConfigCommand::Reload => {
                    here!("RELOAD");
                    let lua = Self::load_config(&self.path).unwrap();
                    self.lua = lua;
                }
            }
        }
        Ok(())
    }
}
