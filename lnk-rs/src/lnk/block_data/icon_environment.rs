use std::io::{self, Cursor, Read};
use thiserror::Error;

use crate::lnk::helpers::{StringReadError, read_c_utf8, read_c_utf16};

#[derive(Debug, Error)]
pub enum IconEnvironmentDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("string read error: {0}")]
    StringRead(#[from] StringReadError),
}

#[derive(Debug, Clone)]
pub struct IconEnvironment {
    /// Path constructed with environment variables (ANSI/code page), NUL-terminated.
    pub target_ansi: String,
    /// Path constructed with environment variables (Unicode), NUL-terminated.
    pub target_unicode: String,
}

impl IconEnvironment {
    /// `data` must point right after BlockSize+BlockSignature.
    /// Reads exactly 260 + 520 bytes as per spec.
    pub fn parse(data: &mut impl Read) -> Result<Self, IconEnvironmentDataBlockParseError> {
        // --- TargetAnsi (260 bytes) ---
        let mut ansi_buf = [0u8; 260];
        data.read_exact(&mut ansi_buf)?;

        // Follow your Tracker style: validate/scan with helper, then take up to first NUL.
        let mut ansi_slice: &[u8] = &ansi_buf;
        read_c_utf8(&mut ansi_slice, /*allow_empty*/ true)?;
        let ansi_len = ansi_buf
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(ansi_buf.len());
        let target_ansi = String::from_utf8_lossy(&ansi_buf[..ansi_len]).into_owned();

        // --- TargetUnicode (520 bytes = 260 UTF-16LE code units) ---
        let mut uni_buf = [0u8; 520];
        data.read_exact(&mut uni_buf)?;
        // Use your UTF-16 helper to read a NUL-terminated wide string from the fixed buffer.
        let mut cur = Cursor::new(&uni_buf[..]);
        let target_unicode = read_c_utf16(&mut cur)?; // stops at first NUL code unit

        Ok(Self {
            target_ansi,
            target_unicode,
        })
    }
}
