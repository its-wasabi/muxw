use eyre::WrapErr;
use std::{fmt::Write, panic};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct Handler;

impl eyre::EyreHandler for Handler {
    fn debug(
        &self,
        error: &(dyn std::error::Error + 'static),
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{error}")?;

        let mut causes = 0;
        let mut source = error.source();
        while let Some(cause) = source {
            write!(f, "\n    {causes}: {cause}")?;
            causes += 1;
            source = cause.source();
        }

        if causes > 0 {
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn init() -> eyre::Result<()> {
    eyre::set_hook(Box::new(|_| Box::new(Handler)))?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("runtime.log")?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Arc::new(log_file))
                .with_target(true)
                .with_ansi(false),
        )
        .try_init()
        .wrap_err("failed to install tracing subscriber")?;

    handle_panics();

    Ok(())
}

pub fn handle_panics() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");
        let location = info
            .location()
            .map_or_else(|| "<unknown>".to_owned(), std::string::ToString::to_string);
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");

        tracing::error!(
            "panic: {message} at {location} [thread {thread}]{}",
            backtrace()
        );

        default_hook(info);
    }));
}

fn backtrace() -> String {
    let mut out = String::new();
    let mut n = 0;
    backtrace::trace(|frame| {
        backtrace::resolve_frame(frame, |sym| {
            let (Some(name), Some(file), Some(line)) = (sym.name(), sym.filename(), sym.lineno())
            else {
                return;
            };
            let Ok(file) = file.strip_prefix(env!("CARGO_MANIFEST_DIR")) else {
                return;
            };
            let name = format!("{name:#}");
            if name.contains("panic_hook") {
                return;
            }
            let _ = write!(out, "\n    {n}: {name} ({}:{line})", file.display());
            n += 1;
        });
        true
    });
    out
}
