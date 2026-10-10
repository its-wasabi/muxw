#[derive(Debug)]
pub enum LogTarget {
    Stdout,
    Stderr,
    File(std::path::PathBuf),
}

impl LogTarget {
    pub fn new(cli_log_to: Option<&str>) -> Result<Self, super::error::Error> {
        let path = match cli_log_to {
            Some("stdout") => return Ok(Self::Stdout),
            Some("stderr") => return Ok(Self::Stderr),
            Some(path) => std::path::PathBuf::from(path),
            None => std::path::PathBuf::from("~/.local/share/muxw/runtime.log"),
        };

        let path = super::to_absolute(super::expand_tilde(path)?)?;
        Ok(Self::File(path))
    }
}

#[derive(Clone, Copy)]
enum DumpTarget {
    Stdout,
    Stderr,
}

struct DumpData {
    path: std::path::PathBuf,
    target: DumpTarget,
}

pub struct LoggerGuard {
    worker_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    dump: Option<DumpData>,
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        // Dropping the worker guard flushes the non-blocking writer. It has to
        // happen before the file is read back, and fields are only dropped
        // after this function returns.
        drop(self.worker_guard.take());

        let Some(dump) = self.dump.take() else {
            return;
        };

        // Nothing sensible to do about IO errors while shutting down.
        if let Ok(mut file) = std::fs::File::open(&dump.path) {
            let _ = match dump.target {
                DumpTarget::Stdout => std::io::copy(&mut file, &mut std::io::stdout()),
                DumpTarget::Stderr => std::io::copy(&mut file, &mut std::io::stderr()),
            };
        }
        let _ = std::fs::remove_file(&dump.path);
    }
}

pub fn init_logging(target: &LogTarget) -> Result<LoggerGuard, super::error::Error> {
    let (path, dump_target) = match target {
        LogTarget::Stdout => (temp_path(), Some(DumpTarget::Stdout)),
        LogTarget::Stderr => (temp_path(), Some(DumpTarget::Stderr)),
        LogTarget::File(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| {
                    super::error::Error::CreateLogDir {
                        path: parent.to_path_buf(),
                        source,
                    }
                })?;
            }
            (path.clone(), None)
        }
    };

    let file = open_log_file(&path)?;
    let (non_blocking, worker_guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .with_writer(non_blocking)
        .with_ansi(dump_target.is_some())
        .try_init()
        .map_err(super::error::Error::InitLogger)?;

    Ok(LoggerGuard {
        worker_guard: Some(worker_guard),
        dump: dump_target.map(|target| DumpData { path, target }),
    })
}

fn temp_path() -> std::path::PathBuf {
    let dir = super::non_empty_env("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    dir.join(format!("muxw-log-dump-{}.tmp", std::process::id()))
}

fn open_log_file(path: &std::path::Path) -> Result<std::fs::File, super::error::Error> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| super::error::Error::OpenLogFile {
            path: path.to_path_buf(),
            source,
        })
}
