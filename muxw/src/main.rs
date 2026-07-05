use clap::Parser;

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

const DEFAULT_CONFIG: &str = /* lua */
    r#"
print("INSIDE LUA")
Mux.bind("W", Mux.motion.focus.up);
Mux.bind("S", Mux.motion.focus.down);
Mux.bind("D", Mux.motion.focus.right);
Mux.bind("A", Mux.motion.focus.left);
print("LUA DONE")
"#;

const NAME: &str = env!("CARGO_PKG_NAME");
const NAME_C: &std::ffi::CStr = unsafe {
    std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(env!("CARGO_PKG_NAME"), "\0").as_bytes())
};

const fn parse_version(version: &str) -> (u32, u32, u32) {
    let bytes = version.as_bytes();
    let mut parts = [0u32; 3];
    let mut current_part = 0;

    let mut i = 0;
    while i < bytes.len() && current_part < 3 {
        match bytes[i] {
            b'0'..=b'9' => {
                parts[current_part] = parts[current_part] * 10 + (bytes[i] - b'0') as u32;
            }
            b'.' => {
                current_part += 1;
            }
            _ => (),
        }
        i += 1;
    }

    (parts[0], parts[1], parts[2])
}
const VERSION: (u32, u32, u32) = parse_version(env!("CARGO_PKG_VERSION"));

mod cli;
mod compositor;
mod config;
mod error;
mod event_loop;
mod input;
mod path;

#[derive(Debug)]
struct Context {
    cli: cli::Cli,
    path: path::Path,
}

impl Context {
    fn new() -> Result<Self, crate::error::PathError> {
        use clap::Parser;
        let cli = cli::Cli::parse();
        cli.process().expect("Failed to process");
        let path = path::Path::new(cli.config.clone())?;
        Ok(Self { cli, path })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Config(config::ConfigRequest),
    ReloadConfig,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context = Context::new()?;

    #[cfg(debug_assertions)]
    {
        here!("{:#?}", context);
    }

    let mut compositr = compositor::Compositor::new(&context)?;

    compositr.run()
}
