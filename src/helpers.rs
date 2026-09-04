pub const fn get_app_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

pub const fn get_app_name_cstr() -> &'static std::ffi::CStr {
    let cargo_pkg_name = env!("CARGO_PKG_NAME");
    cstr::cstr!(cargo_pkg_name)
}

pub const fn get_app_version_major() -> u32 {
    parse_u32(env!("CARGO_PKG_VERSION_MAJOR"))
}

pub const fn get_app_version_minor() -> u32 {
    parse_u32(env!("CARGO_PKG_VERSION_MINOR"))
}

pub const fn get_app_version_patch() -> u32 {
    parse_u32(env!("CARGO_PKG_VERSION_PATCH"))
}

pub const fn get_app_version() -> (u32, u32, u32) {
    (
        get_app_version_major(),
        get_app_version_minor(),
        get_app_version_patch(),
    )
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

pub const DEFAULT_CONFIG: &str = /* lua */
    r#"
print("INSIDE LUA")
Mux.bind("W", Mux.motion.focus.up);
Mux.bind("S", Mux.motion.focus.down);
Mux.bind("D", Mux.motion.focus.right);
Mux.bind("A", Mux.motion.focus.left);
print("LUA DONE")
"#;
