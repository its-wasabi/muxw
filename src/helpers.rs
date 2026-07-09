pub const DEFAULT_CONFIG: &str = /* lua */
    r#"
print("INSIDE LUA")
Mux.bind("W", Mux.motion.focus.up);
Mux.bind("S", Mux.motion.focus.down);
Mux.bind("D", Mux.motion.focus.right);
Mux.bind("A", Mux.motion.focus.left);
print("LUA DONE")
"#;

pub const fn parse_version(version: &str) -> (u32, u32, u32) {
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
