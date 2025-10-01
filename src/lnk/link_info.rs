use std::io::Read;

use bitflags::bitflags;

use crate::lnk::read_u32;

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
}

#[derive(Debug)]
pub struct LinkInfo {}

impl LinkInfo {
    pub fn parse(data: &mut impl Read) -> Result<Self, LinkInfoParseError> {
        let size = read_u32(data)? - 4;
        let mut data = data.take(size as u64);
        let data = &mut data;

        let offsets = LinkOffsets::parse(data)?;

        println!("Offsets: {:?}", offsets);

        let mut remaining_data = Vec::new();
        data.read_to_end(&mut remaining_data)?;
        println!("Remaining data: {:?}", remaining_data);

        Ok(Self {})
    }
}

#[derive(Debug, Default)]
pub struct LinkOffsets {
    header_size: u32,
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

        let mut offsets = Self {
            header_size,
            ..Default::default()
        };

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

        Ok(offsets)
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
