use std::borrow::Cow;

pub use rustbasic_core_macro::RustEmbed;

pub struct EmbeddedFile {
    pub data: Cow<'static, [u8]>,
    pub metadata: Metadata,
}

pub struct Metadata {
    pub last_modified: Option<u64>,
    pub created: Option<u64>,
    pub sha256_hash: [u8; 32],
}
