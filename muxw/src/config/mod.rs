use std::error::Error;

use muxw_event_loop::sources::EventSource;

mod api;

#[derive(Debug, Default)]
pub struct Config {
    pub keyboard_xkb: std::collections::HashMap<
        api::input::keyboard::KeyboardCriteria,
        api::input::keyboard::KeyboardConfig,
    >,
}

pub enum ConfigEvent {
    Reload {
        path: Option<std::path::PathBuf>,
        done: std::sync::mpsc::Sender<Result<(), crate::error::InitError>>,
    },
}

#[derive(Debug)]
pub struct ConfigHandle(std::sync::mpsc::Sender<ConfigEvent>);

#[derive(Debug)]
pub struct PendingConfigRequest(std::sync::mpsc::Receiver<Result<(), crate::error::InitError>>);

#[derive(Debug, Clone)]
pub struct SharedConfig(std::sync::Arc<arc_swap::ArcSwap<Config>>);

pub struct ConfigContext {
    lua: mlua::Lua,
    path: std::path::PathBuf,
    shared: SharedConfig,
    event_rx: std::sync::mpsc::Receiver<ConfigEvent>,
}

impl ConfigHandle {
    fn dispatch(
        &self,
        make_event: impl FnOnce(
            std::sync::mpsc::Sender<Result<(), crate::error::InitError>>,
        ) -> ConfigEvent,
    ) -> Result<PendingConfigRequest, crate::error::InitError> {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let event = make_event(done_tx);
        self.0
            .send(event)
            .map_err(|err| crate::error::InitError::Io {
                action: "send config event",
                path: None,
                source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            })?;
        Ok(PendingConfigRequest(done_rx))
    }
}

impl ConfigHandle {
    pub fn reload(
        &self,
        path: Option<std::path::PathBuf>,
    ) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(|done| ConfigEvent::Reload { path, done })
    }
}

impl PendingConfigRequest {
    pub fn wait(self) -> Result<(), crate::error::InitError> {
        self.0.recv().map_err(|err| crate::error::InitError::Io {
            action: "config thread died before completion operation",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })??;
        Ok(())
    }
}

impl SharedConfig {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(
            Config::default(),
        )))
    }

    // NOTE: Called only from config thread
    fn publish(&self, config: Config) {
        self.0.store(std::sync::Arc::new(config));
    }

    // Called from any thread — one atomic pointer load
    pub fn load(&self) -> arc_swap::Guard<std::sync::Arc<Config>> {
        self.0.load()
    }
}

impl ConfigContext {
    pub fn spawn(
        path: &std::path::Path,
    ) -> Result<(ConfigHandle, SharedConfig), crate::error::InitError> {
        let shared = crate::config::SharedConfig::new();
        let (event_tx, event_rx) = std::sync::mpsc::channel::<ConfigEvent>();
        let (init_done_tx, init_done_rx) =
            std::sync::mpsc::channel::<Result<(), crate::error::InitError>>();

        std::thread::Builder::new()
            .name(String::from("config"))
            .spawn({
                let path = path.to_owned();
                let shared = shared.clone();

                move || {
                    let config_context = match Self::init(path, event_rx, shared) {
                        Ok(config_context) => {
                            init_done_tx.send(Ok(())).unwrap();
                            config_context
                        }
                        Err(err) => {
                            init_done_tx.send(Err(err)).unwrap();
                            return;
                        }
                    };

                    Self::run(config_context);
                }
            });

        init_done_rx
            .recv()
            .map_err(|_| crate::error::InitError::Io {
                action: "config thread died during init",
                path: None,
                source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
            })??;

        let config_handle = ConfigHandle(event_tx);
        Ok((config_handle, shared))
    }

    fn init(
        path: std::path::PathBuf,
        event_rx: std::sync::mpsc::Receiver<ConfigEvent>,
        shared: SharedConfig,
    ) -> Result<Self, crate::error::InitError> {
        let libs = mlua::StdLib::TABLE
            | mlua::StdLib::MATH
            | mlua::StdLib::STRING
            | mlua::StdLib::IO
            | mlua::StdLib::OS;
        let options = mlua::LuaOptions::default();

        let lua =
            mlua::Lua::new_with(libs, options).map_err(|err| crate::error::InitError::Mlua {
                action: "init lua",
                source: err,
            })?;

        lua.set_app_data(Config::default());

        let mux_table = api::create_global_table(&lua)?;
        lua.globals()
            .set("Mux", mux_table)
            .map_err(|err| crate::error::InitError::Mlua {
                action: "set Mux table",
                source: err,
            })?;

        let mut config_context = Self {
            lua,
            path,
            shared,
            event_rx,
        };

        let path = &config_context.path;
        Self::exec_config_file(&config_context, path);

        Ok(config_context)
    }

    fn run(mut self) {
        while let Ok(event) = self.event_rx.recv() {
            here!("Recv event");
            match event {
                ConfigEvent::Reload { path, done } => {
                    if let Some(path) = path {
                        self.path = path;
                    }
                    let path = self.path.clone();
                    done.send(self.exec_config_file(&path));
                }
            }
        }
    }

    fn exec_config_file(&self, path: &std::path::Path) -> Result<(), crate::error::InitError> {
        *self.lua.app_data_mut::<Config>().unwrap() = Config::default();

        let source = Self::read_config_source(path).map_err(|e| crate::error::InitError::Io {
            action: "read config file",
            path: Some(path.to_owned()),
            source: e,
        })?;

        self.lua
            .load(&source)
            .exec()
            .map_err(|e| crate::error::InitError::Mlua {
                action: "execute config file",
                source: e,
            })?;

        // Extract Config out of app_data, wrap in Arc, publish atomically.
        // All other threads see the new config on their next .load().
        // Old Arc<Config> is dropped when all readers are done with it —
        // no reader is ever blocked or invalidated mid-read.
        let mut config = self.lua.app_data_mut::<Config>().unwrap();

        // CHECK: If changing that to default is a good idea, maybe instead of damage tracking you
        // could return from config list of changes for things that need to be explicitly changed
        // and maintain index only for config that are read on use
        let new_config = std::mem::replace(&mut *config, Config::default());
        self.shared.publish(new_config);

        Ok(())
    }
}

// File managing of ConfigContext
impl ConfigContext {
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
