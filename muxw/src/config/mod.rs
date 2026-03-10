#![allow(clippy::unwrap_used)]
use crate::config::api::event;

pub mod api;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub keyboard_xkb: std::collections::HashMap<
        api::input::keyboard::KeyboardCriteria,
        api::input::keyboard::KeyboardConfig,
    >,
}

#[derive(Debug, Clone, Default)]
pub struct SharedConfig(std::sync::Arc<arc_swap::ArcSwap<Config>>);

pub enum ConfigRequest {
    Reload { path: Option<std::path::PathBuf> },
    Clear,
    Event { event: api::event::Event },
}

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

    pub fn reset(&self) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(ConfigRequest::Clear)
    }

    pub fn event(
        &self,
        event: api::event::Event,
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
            action: "config thread died before completion operation",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })?
    }
}

impl ConfigContext {
    pub fn spawn(
        path: &std::path::Path,
    ) -> Result<(ConfigHandle, SharedConfig), crate::error::InitError> {
        let shared = crate::config::SharedConfig::default();

        // TODO: Check if you should use sync_channel or channel
        let (msg_tx, msg_rx) = std::sync::mpsc::sync_channel(0);
        let (init_tx, init_rx) =
            std::sync::mpsc::sync_channel::<Result<(), crate::error::InitError>>(0);

        std::thread::Builder::new()
            .name("config".into())
            .spawn({
                let path = path.to_owned();
                let shared = shared.clone();

                move || match Self::init(path, msg_rx, shared) {
                    Ok(context) => {
                        init_tx.send(Ok(())).unwrap();
                        Self::run(context);
                    }
                    Err(err) => {
                        init_tx.send(Err(err)).unwrap();
                    }
                }
            })
            .map_err(|err| crate::error::InitError::Io {
                action: "spawn config thread",
                path: None,
                source: err,
            })?;

        init_rx.recv().map_err(|_| crate::error::InitError::Io {
            action: "config thread died during init",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })??;

        Ok((ConfigHandle(msg_tx), shared))
    }

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
        lua.set_app_data(api::event::EventManager::default());

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

            ConfigRequest::Clear => self
                .lua
                .app_data_mut::<ConfigState>()
                .ok_or(crate::error::InitError::Mlua {
                    action: "config user data not initialized",
                })
                .map(|mut state| state.clear()),

            // TODO: Move the handlers of each request to related config file maybe make handling
            // these a trait
            ConfigRequest::Event { event } => self
                .lua
                .app_data_mut::<api::event::EventManager>()
                .ok_or(crate::error::InitError::Mlua {
                    action: "event manager not initialized",
                })?
                .call(&self.lua, &event)
                .map_err(|_| crate::error::InitError::Mlua {
                    action: "dispatch event to lua handler",
                }),
        }
    }

    fn exec_config_file(&self) -> Result<(), crate::error::InitError> {
        if let Some(mut state) = self.lua.app_data_mut::<ConfigState>() {
            state.clear();
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

fn mutate_config(lua: &mlua::Lua, f: impl FnOnce(&mut Config)) -> mlua::Result<()> {
    lua.app_data_mut::<ConfigState>()
        .ok_or_else(|| mlua::Error::runtime("Config user data not initialized"))?
        .mutate(f);
    Ok(())
}
