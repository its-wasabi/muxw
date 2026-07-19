use std::io::Write;

#[derive(Debug)]
pub enum LogTarget {
    Stdout,
    Stderr,
    File(std::path::PathBuf),
}

impl LogTarget {
    pub fn new(cli_log_to: Option<&String>) -> Result<LogTarget, crate::error::PathError> {
        match cli_log_to.map(|s| s.as_str()) {
            Some("stdout") => Ok(LogTarget::Stdout),
            Some("stderr") => Ok(LogTarget::Stderr),
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
                Ok(LogTarget::File(absolute))
            }
            None => {
                let default_path = std::path::PathBuf::from("~/.local/share/muxw/runtime.log");
                let expanded = super::expand_tilde(default_path)?;
                Ok(LogTarget::File(expanded))
            }
        }
    }
}

pub struct LoggerGuard {
    worker_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    dump_on_exit: Option<(std::path::PathBuf, bool)>,
}

impl Drop for LoggerGuard {
    fn drop(&mut self) {
        self.worker_guard.take();

        if let Some((path, is_stdout)) = self.dump_on_exit.take() {
            if let Ok(mut file) = std::fs::File::open(&path) {
                if is_stdout {
                    let mut out = std::io::stdout();
                    let _ = std::io::copy(&mut file, &mut out);
                } else {
                    let mut err = std::io::stderr();
                    let _ = std::io::copy(&mut file, &mut err);
                }
            }
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn init_logging(target: &LogTarget) -> LoggerGuard {
    let builder =
        tracing_subscriber::fmt().with_max_level(tracing_subscriber::filter::LevelFilter::TRACE);

    match target {
        // TODO: Extract common logic to helper function and then handle them separately
        LogTarget::Stdout | LogTarget::Stderr => {
            let is_stdout = matches!(target, LogTarget::Stdout);
            let temp_path =
                std::env::temp_dir().join(format!("muxw-logs-{}.tmp", std::process::id()));

            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_path)
                .expect("Failed to open temporary log buffer");

            let (non_blocking, guard) = tracing_appender::non_blocking(file);

            builder.with_writer(non_blocking).with_ansi(true).init();

            LoggerGuard {
                worker_guard: Some(guard),
                dump_on_exit: Some((temp_path, is_stdout)),
            }
        }
        LogTarget::File(file_path) => {
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).expect("Failed to create log directory");
                }
            }

            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .unwrap_or_else(|e| panic!("Failed to open log file at {:?}: {}", file_path, e));

            file.write_all(
                "\n-[NEW-LOG]---------------------------------------------------------\n"
                    .as_bytes(),
            )
            .unwrap();

            let (non_blocking, guard) = tracing_appender::non_blocking(file);

            builder.with_writer(non_blocking).with_ansi(false).init();

            LoggerGuard {
                worker_guard: Some(guard),
                dump_on_exit: None,
            }
        }
    }
}
