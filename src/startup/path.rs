use eyre::{Context, ContextCompat};

#[derive(Debug, Clone)]
pub struct PathEntry {
    pub file: std::path::PathBuf,
    pub dir: std::path::PathBuf,
}

impl PathEntry {
    fn from_dir(dir: std::path::PathBuf, filename: &str) -> eyre::Result<Self> {
        let dir = std::path::absolute(&dir)
            .with_context(|| format!("failed to resolve path: {}", dir.display()))?;
        let file = dir.join(filename);
        Ok(Self { file, dir })
    }

    fn from_user_path(path: std::path::PathBuf, default_filename: &str) -> eyre::Result<Self> {
        use std::os::unix::ffi::OsStrExt;

        let path = expand_tilde(path)?;
        let path = std::path::absolute(&path)
            .with_context(|| format!("failed to resolve path: {}", path.display()))?;

        if path.is_dir() || path.as_os_str().as_bytes().ends_with(b"/") {
            return Ok(Self {
                file: path.join(default_filename),
                dir: path,
            });
        }

        let dir = path
            .parent()
            .with_context(|| format!("path has no parent directory: {}", path.display()))?
            .to_path_buf();

        Ok(Self { file: path, dir })
    }
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub config: PathEntry,
    pub log: PathEntry,
}

impl Paths {
    pub fn new(config_override: Option<std::path::PathBuf>) -> eyre::Result<Self> {
        let config = resolve_entry(
            config_override,
            "MUXW_CONFIG_PATH",
            crate::CONFIG_FILENAME,
            || match xdg_dir("XDG_CONFIG_HOME", ".config") {
                Some(dir) => Ok(dir),
                None => std::env::current_dir().context("failed to determine current directory"),
            },
        )?;

        let log_filename = format!("{}-{}.log", crate::APP_NAME, std::process::id());
        let log = resolve_entry(None, "MUXW_LOG_PATH", &log_filename, || {
            Ok(xdg_dir("XDG_STATE_HOME", ".local/state")
                .unwrap_or_else(|| std::env::temp_dir().join(crate::APP_NAME)))
        })?;

        Ok(Self { config, log })
    }
}

fn resolve_entry(
    cli_override: Option<std::path::PathBuf>,
    env_var: &str,
    default_filename: &str,
    default_dir: impl FnOnce() -> eyre::Result<std::path::PathBuf>,
) -> eyre::Result<PathEntry> {
    if let Some(path) = cli_override {
        return PathEntry::from_user_path(path, default_filename);
    }

    if let Some(value) = non_empty_env(env_var) {
        return PathEntry::from_user_path(std::path::PathBuf::from(value), default_filename);
    }

    PathEntry::from_dir(default_dir()?, default_filename)
}

fn xdg_dir(env_var: &str, home_fallback: &str) -> Option<std::path::PathBuf> {
    if let Some(xdg) = non_empty_env(env_var).map(std::path::PathBuf::from)
        && xdg.is_absolute()
    {
        return Some(xdg.join(crate::APP_NAME));
    }

    let home = std::path::PathBuf::from(non_empty_env("HOME")?);
    home.is_absolute()
        .then(|| home.join(home_fallback).join(crate::APP_NAME))
}

fn non_empty_env(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

fn expand_tilde(path: std::path::PathBuf) -> eyre::Result<std::path::PathBuf> {
    let Some(text) = path.to_str() else {
        return Ok(path);
    };

    if text == "~" {
        return home_dir();
    }

    if let Some(rest) = text.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }

    if text.starts_with('~') {
        eyre::bail!("unsupported tilde expansion: {}", path.display());
    }

    Ok(path)
}

fn home_dir() -> eyre::Result<std::path::PathBuf> {
    non_empty_env("HOME")
        .map(std::path::PathBuf::from)
        .wrap_err("HOME is not set")
}
