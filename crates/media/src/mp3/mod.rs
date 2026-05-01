use crate::formats::{AudioFormat, read_prefix};
use anyhow::Result;
use std::path::Path;

pub static FORMAT: Format = Format;

pub struct Format;

impl AudioFormat for Format {
    fn id(&self) -> &'static str {
        "mp3"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp3"]
    }

    fn mime(&self) -> Option<&'static str> {
        Some("audio/mpeg")
    }

    fn sniff(&self, path: &Path) -> Result<bool> {
        let mut buf = [0u8; 3];
        let n = read_prefix(path, &mut buf)?;
        Ok(n == buf.len() && (&buf == b"ID3" || (buf[0] == 0xff && (buf[1] & 0xe0) == 0xe0)))
    }
}
