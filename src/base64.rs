pub mod engine {
    pub mod general_purpose {
        pub struct Standard;

        impl Standard {
            /// Encodes a byte array to a Base64 string.
            pub fn encode<T: AsRef<[u8]>>(&self, bytes: T) -> String {
                super::super::encode(bytes.as_ref())
            }

            /// Decodes a Base64 string to a byte array.
            pub fn decode<T: AsRef<str>>(&self, s: T) -> Result<Vec<u8>, super::super::DecodeError> {
                super::super::decode(s.as_ref())
            }
        }

        pub const STANDARD: Standard = Standard;
    }
}

#[derive(Debug)]
pub struct DecodeError;

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Format Base64 tidak valid")
    }
}

impl std::error::Error for DecodeError {}

/// Encode a byte slice as standard Base64 with padding.
pub fn encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).map(|&b| b as usize).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as usize).unwrap_or(0);

        output.push(CHARS[b0 >> 2] as char);
        output.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            output.push(CHARS[((b1 & 15) << 2) | (b2 >> 6)] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(CHARS[b2 & 63] as char);
        } else {
            output.push('=');
        }
    }
    output
}

/// Decode a standard Base64 string (ignoring padding).
pub fn decode(input: &str) -> Result<Vec<u8>, DecodeError> {
    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;

    for &c in input.as_bytes() {
        let val = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(DecodeError),
        } as u32;

        buffer = (buffer << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Ok(output)
}
