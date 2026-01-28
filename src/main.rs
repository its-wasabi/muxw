// IMPORTANT: Move all error enums to separate file
#![allow(unused)]
macro_rules! here {
    () => {
        #[cfg(debug_assertions)]
        eprintln!(
            "\x1b[38;5;3m[{}:\x1b[38;5;1m{}\x1b[38;5;3m]\x1b[0m",
            file!(),
            line!()
        )
    };
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!(
        "\x1b[38;5;3m[{}:{}]\x1b[0m {}",
        file!(),
        line!(),
        format_args!($($arg)*)
        )
    };
}

fn exit_on_error<T, E: std::fmt::Display>(result: Result<T, E>, code: i32) -> T {
    result.unwrap_or_else(|err| {
        eprintln!("\x1b[38;5;1mERROR:\x1b[0m {err}");
        std::process::exit(code)
    })
}

const DEFAULT_CONFIG: &str = r#"print("INSIDE LUA")
Ray.bind("W", Ray.motion.focus.up);
Ray.bind("S", Ray.motion.focus.down);
Ray.bind("D", Ray.motion.focus.right);
Ray.bind("A", Ray.motion.focus.left);
print("LUA DONE")
"#;

const NAME: &str = env!("CARGO_PKG_NAME");
const NAME_C: &std::ffi::CStr = unsafe {
    std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(env!("CARGO_PKG_NAME"), "\0").as_bytes())
};

const fn parse_version(version: &str) -> (u32, u32, u32) {
    let bytes = version.as_bytes();
    let mut parts = [0u32; 3];
    let mut current = 0;
    let mut i = 0;

    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'.' {
            current += 1;
            if current >= 3 {
                break;
            }
        } else if byte >= b'0' && byte <= b'9' && current < 3 {
            parts[current] = parts[current] * 10 + (byte - b'0') as u32;
        }
        i += 1;
    }

    (parts[0], parts[1], parts[2])
}
const VERSION: (u32, u32, u32) = parse_version(env!("CARGO_PKG_VERSION"));

static CLI: utils::global::Global<cli::Cli> = utils::global::Global::new();
static PATH: utils::global::Global<path::Path> = utils::global::Global::new();

mod cli;
mod compositor;
mod config;
mod error;
mod path;
mod utils;

fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    match exit_on_error(cli.handle(), 1) {
        cli::CliHandleOutcome::Exit => return,
        cli::CliHandleOutcome::Continue => (),
    }
    let path = exit_on_error(path::Path::new(&cli), 1);
    CLI.init(cli);
    PATH.init(path);

    here!("Cli & Path cfg passed - CLI: {CLI:?}, PATH: {PATH:?}");

    let mut ray = compositor::Compositor::new();
}
