#![allow(unused)]

use clap::Parser;

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

static CLI: std::sync::OnceLock<cli::Cli> = std::sync::OnceLock::new();
static PATH: std::sync::OnceLock<path::Path> = std::sync::OnceLock::new();

mod cli;
mod compositor;
mod path;

fn main() {
    let cli = cli::Cli::parse();
    cli.handle();
    let path = path::Path::new(&cli);
    CLI.set(cli).expect("Failed to set cli arguments");

    println!("Reached");

    let mut ray = compositor::Ray::new();
    ray.run();
}
