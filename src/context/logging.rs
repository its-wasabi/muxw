use std::io::Write;

#[derive(Debug)]
pub enum LogTarget {
    Stdout,
    Stderr,
    File(std::path::PathBuf),
}

impl LogTarget {
    pub fn new(cli_log_to: Option<&String>) -> Result<Self, crate::error::PathError> {
        match cli_log_to.map(std::string::String::as_str) {
            Some("stdout") => Ok(Self::Stdout),
            Some("stderr") => Ok(Self::Stderr),
            Some(path_str) => {
                let p = super::expand_tilde(std::path::PathBuf::from(path_str))?;

                let absolute = if p.is_absolute() {
                    p
                } else {
                    std::env::current_dir()
                        .map_err(|err| crate::error::PathError::Io {
                            action: "get current directory for log path",
                            path: None,
                            source: err,
                        })?
                        .join(p)
                };
                Ok(Self::File(absolute))
            }
            None => {
                let default_path = std::path::PathBuf::from("~/.local/share/muxw/runtime.log");
                let expanded = super::expand_tilde(default_path)?;
                Ok(Self::File(expanded))
            }
        }
    }
}

pub struct LoggerGuard {
    _worker_guard: tracing_appender::non_blocking::WorkerGuard,
    _dump_data: Option<DumpData>,
}
impl LoggerGuard {
    const fn new(worker_guard: tracing_appender::non_blocking::WorkerGuard) -> Self {
        Self {
            _worker_guard: worker_guard,
            _dump_data: None,
        }
    }

    const fn new_with_dump(
        worker_guard: tracing_appender::non_blocking::WorkerGuard,
        path: std::path::PathBuf,
        to_stdout: bool,
    ) -> Self {
        Self {
            _worker_guard: worker_guard,
            _dump_data: Some(DumpData::new(path, to_stdout)),
        }
    }
}

struct DumpData {
    path: std::path::PathBuf,
    to_stdout: bool,
}

impl DumpData {
    const fn new(path: std::path::PathBuf, to_stdout: bool) -> Self {
        Self { path, to_stdout }
    }
}

impl Drop for DumpData {
    fn drop(&mut self) {
        if let Ok(mut file) = std::fs::File::open(&self.path) {
            if self.to_stdout {
                let mut out = std::io::stdout();
                let _ = std::io::copy(&mut file, &mut out);
            } else {
                let mut err = std::io::stderr();
                let _ = std::io::copy(&mut file, &mut err);
            }
        }

        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn init_logging(target: &LogTarget) -> Result<LoggerGuard, Box<dyn std::error::Error>> {
    let builder =
        tracing_subscriber::fmt().with_max_level(tracing_subscriber::filter::LevelFilter::TRACE);

    match target {
        LogTarget::Stdout => {
            let temp_path = temp_path();
            let file = open_log_file(&temp_path)?;
            let (non_blocking, worker_guard) = tracing_appender::non_blocking(file);
            builder.with_writer(non_blocking).with_ansi(true).init();
            Ok(LoggerGuard::new_with_dump(worker_guard, temp_path, true))
        }
        LogTarget::Stderr => {
            let temp_path = temp_path();
            let file = open_log_file(&temp_path)?;
            let (non_blocking, worker_guard) = tracing_appender::non_blocking(file);
            builder.with_writer(non_blocking).with_ansi(true).init();
            Ok(LoggerGuard::new_with_dump(worker_guard, temp_path, false))
        }
        LogTarget::File(file_path) => {
            if let Some(parent) = file_path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)?;
            }

            let mut file = open_log_file(file_path)?;

            file.write_all(
                "\n-[NEW-LOG]---------------------------------------------------------\n"
                    .as_bytes(),
            )?;

            let (non_blocking, worker_guard) = tracing_appender::non_blocking(file);
            builder.with_writer(non_blocking).with_ansi(false).init();

            Ok(LoggerGuard::new(worker_guard))
        }
    }
}

fn temp_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("muxw-log-dump-{}.tmp", std::process::id()))
}

fn open_log_file(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}
