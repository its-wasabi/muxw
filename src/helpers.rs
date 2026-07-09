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
