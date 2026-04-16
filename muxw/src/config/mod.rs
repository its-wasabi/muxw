// pub mod api;

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
        let libs = mlua::StdLib::TABLE
            | mlua::StdLib::MATH
            | mlua::StdLib::STRING
            | mlua::StdLib::IO
            | mlua::StdLib::OS;
        let options = mlua::LuaOptions::default();

        let lua = mlua::Lua::new_with(libs, options)
            .map_err(|err| crate::error::InitError::Mlua { action: "init lua" })?;

        let path = path.to_owned();

        Ok(Self {
            lua,
            path,
            command,
            event,
        })
    }

    fn run(self) {
        while let command_message = self.command.recv() {
            let command_message = command_message.unwrap();
            let command = command_message.0;
            let done_tx = command_message.1;

            match command {
                muxw_types::config::ConfigCommand::Reload => {
                    here!("RELOAD");
                }
            }
        }
    }
}

/*
impl ConfigContext {
    fn init(
        path: std::path::PathBuf,
        event_rx: std::sync::mpsc::Receiver<ConfigMessage>,
        shared: SharedConfig,
    ) -> Result<Self, crate::error::InitError> {
        let libs = mlua::StdLib::TABLE
            | mlua::StdLib::MATH
            | mlua::StdLib::STRING
            | mlua::StdLib::IO
            | mlua::StdLib::OS;
        let options = mlua::LuaOptions::default();

        let lua = mlua::Lua::new_with(libs, options)
            .map_err(|err| crate::error::InitError::Mlua { action: "init lua" })?;

        lua.set_app_data(ConfigState::new(shared.clone()));
        lua.set_app_data(api::event::EventRegistry::default());

        let mux_table = api::create_global_table(&lua)?;
        lua.globals()
            .set("Mux", mux_table)
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set Mux table",
            })?;

        let mut context = Self {
            lua,
            path,
            shared,
            event_rx,
        };

        context.exec_config_file()?;

        Ok(context)
    }

    fn run(mut self) {
        while let Ok(ConfigMessage { request, done }) = self.event_rx.recv() {
            // TODO: Make each callback run in its own thread, preferably make it detect how heavy
            // callback is and create thread depending on that
            let result = self.handle_request(request);
            let _ = done.send(result);
        }
    }

    fn handle_request(&mut self, request: ConfigRequest) -> Result<(), crate::error::InitError> {
        match request {
            ConfigRequest::Reload { path } => {
                if let Some(path) = path {
                    self.path = path;
                }
                self.exec_config_file()
            }

            ConfigRequest::Clear => self.clear_app_data(),

            // TODO: Move the handlers of each request to related config file maybe make handling
            // these a trait
            ConfigRequest::Event { event } => {
                self.lua
                    .app_data_ref::<api::event::EventRegistry>()
                    .ok_or(crate::error::InitError::Mlua {
                        action: "event registry not initialized",
                    })?
                    // FIX: Change that 0.0 into actual timestamp
                    .fire(&self.lua, &(*event), 0.0)
                    .map_err(|_| crate::error::InitError::Mlua {
                        action: "fire event in lua handler",
                    })
            }
        }
    }

    fn clear_app_data(&self) -> Result<(), crate::error::InitError> {
        self.lua
            .app_data_mut::<ConfigState>()
            .ok_or(crate::error::InitError::Mlua {
                action: "ConfigState not initialized",
            })?
            .clear();

        self.lua
            .app_data_mut::<api::event::EventRegistry>()
            .ok_or(crate::error::InitError::Mlua {
                action: "EventRegistry not initialized",
            })?
            .clear();

        Ok(())
    }

    fn exec_config_file(&self) -> Result<(), crate::error::InitError> {
        if let Some(mut state) = self.lua.app_data_mut::<ConfigState>() {
            state.clear();
        }
        if let Some(mut event_registry) = self.lua.app_data_mut::<api::event::EventRegistry>() {
            event_registry.clear();
        }

        let source =
            Self::read_config_source(&self.path).map_err(|err| crate::error::InitError::Io {
                action: "read config file",
                path: Some(self.path.clone()),
                source: err,
            })?;

        // TODO: Unwrap is only temporally until there will be mechanism for handling lua errors
        self.lua.load(&source).exec().unwrap();
        // .map_err(|e| crate::error::InitError::Mlua {
        //     action: "execute config file",
        // })?;

        Ok(())
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
}
*/
/*
struct ConfigMessage {
    request: ConfigRequest,
    done: std::sync::mpsc::SyncSender<Result<(), crate::error::InitError>>,
}

#[derive(Debug)]
pub struct ConfigHandle(std::sync::mpsc::SyncSender<ConfigMessage>);

#[derive(Debug)]
pub struct PendingConfigRequest(std::sync::mpsc::Receiver<Result<(), crate::error::InitError>>);

struct ConfigState {
    local: Config,
    shared: SharedConfig,
}

pub struct ConfigContext {
    lua: mlua::Lua,
    path: std::path::PathBuf,
    shared: SharedConfig,
    event_rx: std::sync::mpsc::Receiver<ConfigMessage>,
}

impl SharedConfig {
    fn store(&self, config: std::sync::Arc<Config>) {
        self.0.store(config);
    }

    pub fn load(&self) -> arc_swap::Guard<std::sync::Arc<Config>> {
        self.0.load()
    }
}

impl ConfigState {
    fn new(shared: SharedConfig) -> Self {
        Self {
            local: Config::default(),
            shared,
        }
    }

    fn mutate(&mut self, f: impl FnOnce(&mut Config)) {
        f(&mut self.local);
        self.shared.store(std::sync::Arc::new(self.local.clone()));
    }

    fn clear(&mut self) {
        self.local = Config::default();
        self.shared.store(std::sync::Arc::new(Config::default()));
    }
}

impl ConfigHandle {
    pub fn reload(
        &self,
        path: Option<std::path::PathBuf>,
    ) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(ConfigRequest::Reload { path })
    }

    pub fn clear(&self) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(ConfigRequest::Clear)
    }

    pub fn event(
        &self,
        event: Box<dyn api::event::ErasedEventKind>,
    ) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(ConfigRequest::Event { event })
    }

    fn dispatch(
        &self,
        request: ConfigRequest,
    ) -> Result<PendingConfigRequest, crate::error::InitError> {
        let (done, done_rx) = std::sync::mpsc::sync_channel(1);

        self.0
            .send(ConfigMessage { request, done })
            .map_err(|err| crate::error::InitError::Io {
                action: "send config event",
                path: None,
                source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            })?;
        Ok(PendingConfigRequest(done_rx))
    }
}

impl PendingConfigRequest {
    pub fn wait(self) -> Result<(), crate::error::InitError> {
        self.0.recv().map_err(|err| crate::error::InitError::Io {
            action: "config thread died before completing operation",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })?
    }
}


fn mutate_config(lua: &mlua::Lua, f: impl FnOnce(&mut Config)) -> mlua::Result<()> {
    lua.app_data_mut::<ConfigState>()
        .ok_or_else(|| mlua::Error::runtime("Config user data not initialized"))?
        .mutate(f);
    Ok(())
}
*/
