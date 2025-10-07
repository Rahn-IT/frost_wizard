use std::io::{self, Read};

use thiserror::Error;

use crate::lnk::helpers::{Guid, StringReadError, read_c_utf8, read_guid, read_u32};

#[derive(Debug, Error)]
pub enum TrackerDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid Length: expected 0x00000058, got 0x{0:08X}")]
    InvalidLength(u32),
    #[error("invalid Version: expected 0x00000000, got 0x{0:08X}")]
    InvalidVersion(u32),
    #[error("string read error: {0}")]
    StringReadError(#[from] StringReadError),
}

#[derive(Debug, Clone)]
pub struct TrackerDataBlock {
    /// `machine_id_raw` decoded lossily as UTF-8 (up to first NUL).
    pub machine_id: String,
    /// Two GUIDs used by the Link Tracking service.
    pub droid: (Guid, Guid),
    /// Two GUIDs used by the Link Tracking service (birth).
    pub droid_birth: (Guid, Guid),
}

impl TrackerDataBlock {
    pub fn parse(data: &mut impl Read) -> Result<Self, TrackerDataBlockParseError> {
        // Length (MUST be 0x58) and Version (MUST be 0)
        let length = read_u32(data)?;
        if length != 0x0000_0058 {
            return Err(TrackerDataBlockParseError::InvalidLength(length));
        }
        let version = read_u32(data)?;
        if version != 0x0000_0000 {
            return Err(TrackerDataBlockParseError::InvalidVersion(version));
        }

        // MachineID: 16-byte NUL-terminated SBCS buffer (NetBIOS name).
        let mut machine_id_raw = [0u8; 16];
        data.read_exact(&mut machine_id_raw)?;
        let mut machine_id_raw = machine_id_raw.as_slice();
        read_c_utf8(&mut machine_id_raw, false)?;
        let nul = machine_id_raw
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(machine_id_raw.len());
        let machine_id = String::from_utf8_lossy(&machine_id_raw[..nul]).into_owned();

        // Droid: two GUIDs
        let droid = (read_guid(data)?, read_guid(data)?);

        // DroidBirth: two GUIDs
        let droid_birth = (read_guid(data)?, read_guid(data)?);

        Ok(Self {
            machine_id,
            droid,
            droid_birth,
        })
    }
}
