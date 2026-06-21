use std::fs::File;
use std::io::Read;

/// Fill the buffer with cryptographically secure random bytes from /dev/urandom.
/// If unavailable (e.g. non-Unix sandbox), fall back to a system-time-seeded LCG generator.
pub fn fill_bytes(buf: &mut [u8]) {
    if let Ok(mut f) = File::open("/dev/urandom")
        && f.read_exact(buf).is_ok() {
            return;
        }
    
    // Fallback: LCG generator using system time as seed
    use std::time::SystemTime;
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0xFEEDFACE_DEADC0DE);
        
    for byte in buf.iter_mut() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *byte = (seed >> 56) as u8;
    }
}

/// Generate a random alphanumeric string of the specified length.
pub fn random_alphanumeric(length: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut bytes = vec![0u8; length];
    fill_bytes(&mut bytes);
    let mut s = String::with_capacity(length);
    for b in bytes {
        let idx = (b as usize) % CHARS.len();
        s.push(CHARS[idx] as char);
    }
    s
}

/// Custom random number generator struct for backward compatibility
#[derive(Clone, Copy, Debug)]
pub struct CustomRng;

impl CustomRng {
    pub fn fill_bytes(&self, buf: &mut [u8]) {
        fill_bytes(buf);
    }
}

pub fn rng() -> CustomRng {
    CustomRng
}

pub mod distr {
    pub struct Alphanumeric;

    pub trait SampleString {
        fn sample_string(&self, rng: &mut super::CustomRng, length: usize) -> String;
    }

    impl SampleString for Alphanumeric {
        fn sample_string(&self, _rng: &mut super::CustomRng, length: usize) -> String {
            super::random_alphanumeric(length)
        }
    }
}
