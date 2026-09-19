pub const fn get_app_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub const fn get_app_name_cstr() -> &'static std::ffi::CStr {
    let bytes = concat!(env!("CARGO_PKG_NAME"), "\0").as_bytes();
    unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(bytes) }
}

#[derive(Debug, Clone, Copy)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub const fn get_app_version() -> Version {
    Version {
        major: parse_u32(env!("CARGO_PKG_VERSION_MAJOR")),
        minor: parse_u32(env!("CARGO_PKG_VERSION_MINOR")),
        patch: parse_u32(env!("CARGO_PKG_VERSION_PATCH")),
    }
}

const fn parse_u32(s: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut result = 0u32;
    let mut i = 0;
    while i < bytes.len() {
        result = result * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    result
}
