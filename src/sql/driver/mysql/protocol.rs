use std::io::{Read, Write};
use std::net::TcpStream;
use crate::sql::driver::error::SqlError;

// SHA-1 Implementation from Scratch
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let mut msg = data.to_vec();
    let orig_len = msg.len() as u64;
    msg.push(0x80);
    while (msg.len() + 8) % 64 != 0 {
        msg.push(0x00);
    }
    let bit_len = orig_len * 8;
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            let val = w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16];
            w[i] = val.rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

// SHA-256 Implementation from Scratch
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut msg = data.to_vec();
    let orig_len = msg.len() as u64;
    msg.push(0x80);
    while (msg.len() + 8) % 64 != 0 {
        msg.push(0x00);
    }
    let bit_len = orig_len * 8;
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..(i + 1) * 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

// MySQL Native Password Hash: XOR( SHA1(pwd), SHA1(salt + SHA1(SHA1(pwd))) )
pub fn mysql_native_password_hash(password: &[u8], salt: &[u8]) -> Vec<u8> {
    let h1 = sha1(password);
    let h2 = sha1(&h1);
    
    let mut concat = Vec::new();
    concat.extend_from_slice(salt);
    concat.extend_from_slice(&h2);
    let h3 = sha1(&concat);
    
    let mut reply = vec![0u8; 20];
    for i in 0..20 {
        reply[i] = h1[i] ^ h3[i];
    }
    reply
}

// MySQL Caching SHA2 Password Hash
pub fn caching_sha2_password_hash(password: &[u8], salt: &[u8]) -> Vec<u8> {
    let h1 = sha256(password);
    let h2 = sha256(&h1);
    
    let mut concat = Vec::new();
    concat.extend_from_slice(salt);
    concat.extend_from_slice(&h2);
    let h3 = sha256(&concat);
    
    let mut reply = vec![0u8; 32];
    for i in 0..32 {
        reply[i] = h1[i] ^ h3[i];
    }
    reply
}

// Reading MySQL packets
pub fn read_packet(stream: &mut TcpStream) -> Result<(Vec<u8>, u8), SqlError> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;
    let length = (header[0] as usize) | ((header[1] as usize) << 8) | ((header[2] as usize) << 16);
    let seq_id = header[3];
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload)?;
    Ok((payload, seq_id))
}

// Writing MySQL packets
pub fn write_packet(stream: &mut TcpStream, seq_id: u8, payload: &[u8]) -> Result<(), SqlError> {
    let length = payload.len();
    let header = [
        (length & 0xFF) as u8,
        ((length >> 8) & 0xFF) as u8,
        ((length >> 16) & 0xFF) as u8,
        seq_id,
    ];
    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

// Length-encoded integer parsing
pub fn read_lenenc_int(cursor: &mut &[u8]) -> Result<Option<u64>, SqlError> {
    if cursor.is_empty() {
        return Err(SqlError::Protocol("Unexpected EOF reading length-encoded integer".into()));
    }
    let first = cursor[0];
    *cursor = &cursor[1..];
    match first {
        0..=250 => Ok(Some(first as u64)),
        251 => Ok(None), // NULL
        252 => {
            if cursor.len() < 2 {
                return Err(SqlError::Protocol("Unexpected EOF reading 2-byte integer".into()));
            }
            let val = u16::from_le_bytes([cursor[0], cursor[1]]) as u64;
            *cursor = &cursor[2..];
            Ok(Some(val))
        }
        253 => {
            if cursor.len() < 3 {
                return Err(SqlError::Protocol("Unexpected EOF reading 3-byte integer".into()));
            }
            let val = (cursor[0] as u64) | ((cursor[1] as u64) << 8) | ((cursor[2] as u64) << 16);
            *cursor = &cursor[3..];
            Ok(Some(val))
        }
        254 => {
            if cursor.len() < 8 {
                return Err(SqlError::Protocol("Unexpected EOF reading 8-byte integer".into()));
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&cursor[0..8]);
            let val = u64::from_le_bytes(bytes);
            *cursor = &cursor[8..];
            Ok(Some(val))
        }
        255 => Err(SqlError::Protocol("Invalid length-encoded integer marker 0xFF".into())),
    }
}

// Length-encoded string parsing
pub fn read_lenenc_str(cursor: &mut &[u8]) -> Result<Option<String>, SqlError> {
    match read_lenenc_int(cursor)? {
        Some(len) => {
            let len = len as usize;
            if cursor.len() < len {
                return Err(SqlError::Protocol(format!(
                    "Unexpected EOF reading length-encoded string: need {} bytes, have {}",
                    len,
                    cursor.len()
                )));
            }
            let s_bytes = &cursor[..len];
            *cursor = &cursor[len..];
            let s = String::from_utf8(s_bytes.to_vec())
                .map_err(|e| SqlError::Decode(format!("Invalid UTF-8 in length-encoded string: {}", e)))?;
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

// Length-encoded bytes parsing
pub fn read_lenenc_bytes(cursor: &mut &[u8]) -> Result<Option<Vec<u8>>, SqlError> {
    match read_lenenc_int(cursor)? {
        Some(len) => {
            let len = len as usize;
            if cursor.len() < len {
                return Err(SqlError::Protocol(format!(
                    "Unexpected EOF reading length-encoded bytes: need {} bytes, have {}",
                    len,
                    cursor.len()
                )));
            }
            let bytes = &cursor[..len];
            *cursor = &cursor[len..];
            Ok(Some(bytes.to_vec()))
        }
        None => Ok(None),
    }
}

// Writing length-encoded integer
pub fn write_lenenc_int(buf: &mut Vec<u8>, val: u64) {
    if val <= 250 {
        buf.push(val as u8);
    } else if val <= 0xFFFF {
        buf.push(252);
        buf.extend_from_slice(&(val as u16).to_le_bytes());
    } else if val <= 0xFFFFFF {
        buf.push(253);
        buf.push((val & 0xFF) as u8);
        buf.push(((val >> 8) & 0xFF) as u8);
        buf.push(((val >> 16) & 0xFF) as u8);
    } else {
        buf.push(254);
        buf.extend_from_slice(&val.to_le_bytes());
    }
}

// Writing length-encoded string
pub fn write_lenenc_str(buf: &mut Vec<u8>, val: &str) {
    let bytes = val.as_bytes();
    write_lenenc_int(buf, bytes.len() as u64);
    buf.extend_from_slice(bytes);
}

// Null-terminated string parsing
pub fn read_null_terminated_str(cursor: &mut &[u8]) -> Result<String, SqlError> {
    if let Some(pos) = cursor.iter().position(|&b| b == 0) {
        let s_bytes = &cursor[..pos];
        *cursor = &cursor[pos + 1..];
        let s = String::from_utf8(s_bytes.to_vec())
            .map_err(|e| SqlError::Decode(format!("Invalid UTF-8 in null-terminated string: {}", e)))?;
        Ok(s)
    } else {
        Err(SqlError::Protocol("Could not find null terminator for string".into()))
    }
}

// Parse ERR packet payload to SqlError
pub fn parse_err_payload(payload: &[u8]) -> SqlError {
    if payload.is_empty() || payload[0] != 0xFF {
        return SqlError::Protocol("Invalid ERR packet header".into());
    }
    let mut cursor = &payload[1..];
    if cursor.len() < 2 {
        return SqlError::Protocol("Truncated ERR packet".into());
    }
    let code = u16::from_le_bytes([cursor[0], cursor[1]]);
    cursor = &cursor[2..];
    let mut sql_state = String::from("HY000");
    if !cursor.is_empty() && cursor[0] == b'#' {
        if cursor.len() < 6 {
            return SqlError::Protocol("Truncated ERR packet SQL state".into());
        }
        sql_state = String::from_utf8_lossy(&cursor[1..6]).into_owned();
        cursor = &cursor[6..];
    }
    let message = String::from_utf8_lossy(cursor).into_owned();
    SqlError::Server { code, sql_state, message }
}

// Encrypt password using RSA-OAEP via host's openssl command
pub fn rsa_encrypt_password(
    password: &str,
    salt: &[u8],
    pub_key_pem: &str,
) -> Result<Vec<u8>, SqlError> {
    use std::process::{Command, Stdio};
    use std::io::Write as _;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let mut xored = Vec::new();
    let mut pwd_bytes = password.as_bytes().to_vec();
    pwd_bytes.push(0); // trailing null

    for (i, &b) in pwd_bytes.iter().enumerate() {
        xored.push(b ^ salt[i % salt.len()]);
    }

    let temp_dir = std::env::temp_dir();
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    let filename = format!("mysql_pub_key_{}_{}.pem", std::process::id(), count);
    let key_path = temp_dir.join(filename);
    {
        let mut f = std::fs::File::create(&key_path)?;
        f.write_all(pub_key_pem.as_bytes())?;
    }

    let mut child = Command::new("openssl")
        .args(&[
            "pkeyutl",
            "-encrypt",
            "-pubin",
            "-inkey",
            key_path.to_str().unwrap(),
            "-pkeyopt",
            "rsa_padding_mode:oaep",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SqlError::Protocol(format!("Failed to spawn openssl: {}", e)))?;

    {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            SqlError::Protocol("Failed to open openssl stdin".into())
        })?;
        stdin.write_all(&xored)?;
    }

    let output = child.wait_with_output()?;
    let _ = std::fs::remove_file(&key_path);

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(SqlError::Protocol(format!("OpenSSL encryption failed: {}", err_msg)));
    }

    Ok(output.stdout)
}
