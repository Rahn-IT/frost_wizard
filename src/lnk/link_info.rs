use std::io::Read;

use bitflags::bitflags;

use crate::lnk::helpers::{StringReadError, read_c_utf8, read_c_utf16, read_u32};

#[derive(Debug, thiserror::Error)]
pub enum LinkInfoParseError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("The header contained data that this application cannot parse yet")]
    UnreadHeaderData,
    #[error("Invalid link info flags")]
    InvalidFlags,
    #[error("Invalid offset")]
    InvalidOffset,
    #[error("Error reading string: {0}")]
    StringReadError(#[from] StringReadError),
    #[error("Volume ID parse error: {0}")]
    VolumeIdParseError(#[from] VolumeIdParseError),
    #[error("Relative network link unsupported")]
    RelativeNetworkLinkUnsupported,
}

#[derive(Debug)]
pub struct LinkInfo {
    pub volume_id: Option<VolumeId>,
    pub local_base_path: Option<String>,
    pub common_path_suffix: Option<String>,
}

impl LinkInfo {
    pub fn parse(data: &mut impl Read) -> Result<Self, LinkInfoParseError> {
        let size = read_u32(data)? - 4;
        let mut data = data.take(size as u64);
        let data = &mut data;

        let offsets = LinkOffsets::parse(data)?;

        println!("Offsets: {:#?}", offsets);

        let mut remaining_data = Vec::new();
        data.read_to_end(&mut remaining_data)?;
        println!("Remaining: {:?}", remaining_data);

        let data = &mut remaining_data;

        let volume_id = if let Some(offset) = offsets.volume_id {
            let mut data = &data[offset as usize..];
            Some(VolumeId::parse(&mut data)?)
        } else {
            None
        };

        let local_base_path = if let Some(offset) = offsets.local_base_path_unicode {
            let mut data = &data[offset as usize..];
            Some(read_c_utf16(&mut data)?)
        } else if let Some(offset) = offsets.local_base_path {
            let mut data = &data[offset as usize..];
            Some(read_c_utf8(&mut data, false)?)
        } else {
            None
        };

        let common_path_suffix = if let Some(offset) = offsets.common_path_suffix_unicode {
            let mut data = &data[offset as usize..];
            Some(read_c_utf16(&mut data)?)
        } else {
            let offset = offsets.common_path_suffix;
            let mut data = &data[offset as usize..];
            Some(read_c_utf8(&mut data, false)?)
        };

        if let Some(_offset) = offsets.common_network_relative_link {
            return Err(LinkInfoParseError::RelativeNetworkLinkUnsupported);
        }

        println!("volume id: {:?}", volume_id);

        Ok(Self {
            volume_id,
            local_base_path,
            common_path_suffix,
        })
    }
}

#[derive(Debug, Default)]
pub struct LinkOffsets {
    // Local
    volume_id: Option<u32>,
    local_base_path: Option<u32>,
    local_base_path_unicode: Option<u32>,

    // Common
    common_path_suffix: u32,
    common_path_suffix_unicode: Option<u32>,
    common_network_relative_link: Option<u32>,
}

impl LinkOffsets {
    fn parse(data: &mut impl Read) -> Result<Self, LinkInfoParseError> {
        let header_size = read_u32(data)?;
        let mut data = data.take(header_size as u64 - 8);
        let data = &mut data;

        let link_info_flags = read_u32(data)?;
        let link_info_flags = LinkInfoFlags::from_bits(link_info_flags)
            .ok_or_else(|| LinkInfoParseError::InvalidFlags)?;

        let mut offsets = Self::default();

        if link_info_flags.contains(LinkInfoFlags::VOLUME_ID_AND_LOCAL_BASE_PATH) {
            offsets.volume_id = Some(read_u32(data)?);
            offsets.local_base_path = Some(read_u32(data)?);
        } else {
            if read_u32(data)? != 0 {
                return Err(LinkInfoParseError::InvalidOffset);
            }
            if read_u32(data)? != 0 {
                return Err(LinkInfoParseError::InvalidOffset);
            }
        }

        if link_info_flags.contains(LinkInfoFlags::COMMON_NETWORK_RELATIVE_LINK_AND_PATH_SUFFIX) {
            offsets.common_network_relative_link = Some(read_u32(data)?);
        } else {
            if read_u32(data)? != 0 {
                return Err(LinkInfoParseError::InvalidOffset);
            }
        }

        offsets.common_path_suffix = read_u32(data)?;

        println!("Header size: {header_size:0x}");

        if header_size == 0x24 {
            if link_info_flags.contains(LinkInfoFlags::VOLUME_ID_AND_LOCAL_BASE_PATH) {
                offsets.local_base_path_unicode = Some(read_u32(data)?);
            } else {
                if read_u32(data)? != 0 {
                    return Err(LinkInfoParseError::InvalidOffset);
                }
            }

            if link_info_flags.contains(LinkInfoFlags::COMMON_NETWORK_RELATIVE_LINK_AND_PATH_SUFFIX)
            {
                offsets.common_path_suffix_unicode = Some(read_u32(data)?);
            } else {
                if read_u32(data)? != 0 {
                    return Err(LinkInfoParseError::InvalidOffset);
                }
            }
        }

        let mut remaining_data = Vec::new();
        if data.read_to_end(&mut remaining_data)? > 0 {
            println!("Remaining data: {:?}", remaining_data);
            return Err(LinkInfoParseError::UnreadHeaderData);
        }

        offsets.sub(header_size);

        Ok(offsets)
    }

    fn sub(&mut self, value: u32) {
        if let Some(offset) = &mut self.volume_id {
            *offset -= value;
        }
        if let Some(offset) = &mut self.local_base_path {
            *offset -= value;
        }
        if let Some(offset) = &mut self.local_base_path_unicode {
            *offset -= value;
        }

        self.common_path_suffix -= value;
        if let Some(offset) = &mut self.common_path_suffix_unicode {
            *offset -= value;
        }
        if let Some(offset) = &mut self.common_network_relative_link {
            *offset -= value;
        }
    }
}

bitflags! {
    /// The LinkFlags structure defines bits that specify which shell link structures are present in the file
    /// format after the ShellLinkHeader structure (section 2.1).
    #[derive(Debug, Clone)]
    struct LinkInfoFlags: u32 {
        const VOLUME_ID_AND_LOCAL_BASE_PATH                = 0b0000_0000_0000_0000_0000_0000_0000_0001;

        const COMMON_NETWORK_RELATIVE_LINK_AND_PATH_SUFFIX = 0b0000_0000_0000_0000_0000_0000_0000_0010;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VolumeIdParseError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid drive type: {0}")]
    InvalidDriveType(u32),
    #[error("Error reading string: {0}")]
    StringReadError(#[from] StringReadError),
}

#[derive(Debug)]
pub struct VolumeId {
    pub drive_type: DriveType,
    pub serial_number: u32,
    pub label: String,
}

impl VolumeId {
    pub fn parse(data: &mut impl Read) -> Result<Self, VolumeIdParseError> {
        let size = read_u32(data)?;
        let mut data = data.take(size as u64);
        let data = &mut data;

        let drive_type = read_u32(data)?;
        let drive_type = DriveType::from_u32(drive_type)
            .ok_or_else(|| VolumeIdParseError::InvalidDriveType(drive_type))?;

        let serial_number = read_u32(data)?;

        let label_offset = read_u32(data)?;
        let label_unicode_offset = if label_offset == 0x14 {
            Some(read_u32(data)?)
        } else {
            None
        };

        let mut remaining_data = Vec::new();
        data.read_to_end(&mut remaining_data)?;

        let label = if let Some(label_unicode_offset) = label_unicode_offset {
            let label_unicode_offset = label_unicode_offset - 20;
            let mut data = &remaining_data[label_unicode_offset as usize..];
            read_c_utf16(&mut data)?
        } else {
            let label_offset = label_offset - 16;
            let mut data = &remaining_data[label_offset as usize..];
            read_c_utf8(&mut data, false)?
        };

        Ok(VolumeId {
            drive_type,
            serial_number,
            label,
        })
    }
}

#[derive(Debug)]
pub enum DriveType {
    Unknown,
    NoRootDir,
    Removable,
    Fixed,
    Remote,
    CdRom,
    RamDisk,
}

impl DriveType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(DriveType::Unknown),
            1 => Some(DriveType::NoRootDir),
            2 => Some(DriveType::Removable),
            3 => Some(DriveType::Fixed),
            4 => Some(DriveType::Remote),
            5 => Some(DriveType::CdRom),
            6 => Some(DriveType::RamDisk),
            _ => None,
        }
    }
}
