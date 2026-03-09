#![allow(clippy::unwrap_used)]
use crate::config::api::event;

mod api;

fn mutate_config(lua: &mlua::Lua, f: impl FnOnce(&mut Config)) -> mlua::Result<()> {
    lua.app_data_mut::<ConfigState>()
        .ok_or_else(|| mlua::Error::runtime("Config user data not initialized"))?
        .mutate(f);
    Ok(())
}

pub struct ConfigContext {
    lua: mlua::Lua,
    path: std::path::PathBuf,
    shared: SharedConfig,
    event_rx: std::sync::mpsc::Receiver<ConfigEvent>,
}

struct ConfigState {
    local: Config,
    shared: SharedConfig,
}

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub keyboard_xkb: std::collections::HashMap<
        api::input::keyboard::KeyboardCriteria,
        api::input::keyboard::KeyboardConfig,
    >,
}

#[derive(Debug, Clone)]
pub struct SharedConfig(std::sync::Arc<arc_swap::ArcSwap<Config>>);

pub enum ConfigEvent {
    Reload {
        path: Option<std::path::PathBuf>,
        done: std::sync::mpsc::Sender<Result<(), crate::error::InitError>>,
    },
    Clear {
        done: std::sync::mpsc::Sender<Result<(), crate::error::InitError>>,
    },
    Event {
        event: api::event::Event,
        done: std::sync::mpsc::Sender<Result<(), crate::error::InitError>>,
    },
}

#[derive(Debug)]
pub struct ConfigHandle(std::sync::mpsc::Sender<ConfigEvent>);

#[derive(Debug)]
pub struct PendingConfigRequest(std::sync::mpsc::Receiver<Result<(), crate::error::InitError>>);

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
            })
            .map_err(|err| crate::error::InitError::Mlua { action: "whoops" })?;

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

        let mut config_context = Self {
            lua,
            path,
            shared,
            event_rx,
        };

        Self::exec_config_file(&config_context)?;

        Ok(config_context)
    }

    fn run(mut self) {
        while let Ok(event) = self.event_rx.recv() {
            match event {
                ConfigEvent::Reload { path, done } => {
                    if let Some(path) = path {
                        self.path = path;
                    }

                    // NOTE: You don't need to handle error here cause it is send to the caller
                    done.send(self.exec_config_file()).unwrap();
                }
                ConfigEvent::Clear { done } => {
                    if let Some(mut state) = self.lua.app_data_mut::<ConfigState>() {
                        state.clear();
                        done.send(Ok(())).unwrap();
                    } else {
                        done.send(Err(crate::error::InitError::Mlua {
                            action: "config user data not initialized",
                        }));
                    }
                }
                ConfigEvent::Event { event, done } => {
                    let event_manager = self
                        .lua
                        .app_data_mut::<api::event::EventManager>()
                        .ok_or(crate::error::InitError::Mlua {
                            action: "event manager user data not initialized",
                        })
                        .unwrap();

                    event_manager.call(&self.lua, &event);
                }
            }
        }
    }

    #[must_use = "You should handle error variant of the Result"]
    fn exec_config_file(&self) -> Result<(), crate::error::InitError> {
        if let Some(mut state) = self.lua.app_data_mut::<ConfigState>() {
            state.clear();
        }

        let source =
            Self::read_config_source(&self.path).map_err(|e| crate::error::InitError::Io {
                action: "read config file",
                path: Some(self.path.clone()),
                source: e,
            })?;

        self.lua
            .load(&source)
            .exec()
            .map_err(|e| crate::error::InitError::Mlua {
                action: "execute config file",
            })?;

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

impl SharedConfig {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(arc_swap::ArcSwap::from_pointee(
            Config::default(),
        )))
    }

    fn store(&self, config: std::sync::Arc<Config>) {
        self.0.store(config);
    }

    pub fn load(&self) -> arc_swap::Guard<std::sync::Arc<Config>> {
        self.0.load()
    }
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

    pub fn reset(&self) -> Result<PendingConfigRequest, crate::error::InitError> {
        self.dispatch(|done| ConfigEvent::Clear { done })
    }
}

impl PendingConfigRequest {
    #[must_use = "May return important error"]
    pub fn wait(self) -> Result<(), crate::error::InitError> {
        self.0.recv().map_err(|err| crate::error::InitError::Io {
            action: "config thread died before completion operation",
            path: None,
            source: std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        })??;
        Ok(())
    }
}
