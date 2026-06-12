#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Uuid(String);

impl Uuid {
    /// Generate a new version-4 UUID, setting version to 4 and variant to 1.
    pub fn new_v4() -> Self {
        let mut bytes = [0u8; 16];
        crate::rand::fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4
        bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 1

        let s = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        );
        Self(s)
    }

    /// Parse and validate a UUID string
    pub fn parse_str(s: &str) -> Result<Self, &'static str> {
        if s.len() != 36 {
            return Err("Invalid length");
        }
        let bytes = s.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                if b != b'-' {
                    return Err("Invalid separator");
                }
            } else {
                if !b.is_ascii_hexdigit() {
                    return Err("Invalid character");
                }
            }
        }
        Ok(Self(s.to_string()))
    }
}

impl std::fmt::Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
