use std::io::{self, Read, Write};

use chrono::NaiveDateTime;

use crate::lnk::{
    helpers::{
        read_c_utf16, read_c_utf8, read_dos_datetime, read_guid, read_u16, read_u32, read_u64, read_u8, write_dos_datetime, write_u16, write_u32, write_u8, DosDateTimeReadError, Guid, StringReadError
    }, LnkParseError, LnkWriteError
};

#[derive(Debug, thiserror::Error)]
pub enum IdListParseError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("IdList exists, but is empty")]
    ListEmpty,
    #[error("First item in IdList is empty")]
    FirstItemEmpty,
    #[error("Missing root")]
    MissingRoot,
    #[error("Missing drive letter")]
    MissingDrive,
    #[error("Drive entry is invalid")]
    InvalidDriveEntry,
    #[error("Unknown root type")]
    InvalidRootType,
    #[error("Root type not supported yet")]
    UnsupportedRootType,
    #[error("Uwp Paths elements not supported yet")]
    UwpUnsupported,
    #[error("Found invalid entry type {0:0x}")]
    InvalidEntryType(u16),
    #[error("entry type not supported yet")]
    UnsupportedEntryType,
    #[error("error reading string: {0}")]
    StringReadError(#[from] StringReadError),
    #[error("error reading DOS datetime: {0}")]
    DosTimeError(#[from] DosDateTimeReadError),
    #[error("invalid type after drive")]
    InvalidAfterDrive,
    #[error("invalid type after folder")]
    InvalidAfterFolder,
    #[error("entry after file is not allowed")]
    AnyAfterFile,
    #[error("not all bytes read - not fully parsed")]
    BytesLeft,
}

#[derive(Debug)]
pub struct IdList {
    id_list: Vec<IdEntry>,
}

impl IdList {
    pub fn parse(data: &mut impl Read) -> Result<Self, IdListParseError> {
        let size = read_u16(data)?;
        let list_data = &mut data.take(size as u64);
        let mut raw_list_items = Vec::new();

        loop {
            let item_length = read_u16(list_data)?;
            if item_length == 0 {
                break;
            }
            let item_length = item_length as usize - 2;
            let mut item_data = vec![0u8; item_length];
            list_data.read_exact(&mut item_data)?;
            raw_list_items.push(item_data);
        }

        let mut id_list = Vec::new();

        for item in raw_list_items.iter() {
            if let Some(uwp_marker) = item.get(4..8) {
                if uwp_marker == b"APPS" {
                    return Err(IdListParseError::UwpUnsupported);
                }
            }
            let mut data = item.as_slice();

            let id_entry = IdEntry::parse(&mut data)?;
            match id_list.last() {
                None => match id_entry {
                    IdEntry::Root(RootLocationType::MyComputer) => (),
                    IdEntry::Root(_) => return Err(IdListParseError::UnsupportedRootType),
                    _ => return Err(IdListParseError::MissingRoot),
                },
                Some(IdEntry::Root(_)) => match id_entry {
                    IdEntry::Drive(_) => (),
                    _ => return Err(IdListParseError::MissingDrive),
                },
                Some(IdEntry::Drive(_)) => match id_entry {
                    IdEntry::Folder(_) | IdEntry::File(_) => (),
                    _ => return Err(IdListParseError::InvalidAfterDrive),
                },
                Some(IdEntry::Folder(_)) => match id_entry {
                    IdEntry::Folder(_) | IdEntry::File(_) => (),
                    _ => return Err(IdListParseError::InvalidAfterFolder),
                },
                Some(IdEntry::File(_)) => return Err(IdListParseError::AnyAfterFile),
            }
            id_list.push(id_entry);
        }

        match id_list.last() {
            None => return Err(IdListParseError::ListEmpty),
            Some(IdEntry::Root(_)) => return Err(IdListParseError::MissingDrive),
            _ => (),
        }

        let mut left = Vec::new();
        let read = list_data.read_to_end(&mut left)?;
        if read != 0 {
            return Err(IdListParseError::BytesLeft);
        }

        Ok(Self { id_list })
    }

    pub(crate) fn write(&self, data: &mut impl Write) -> Result<(), LnkWriteError> {
        let mut buffer = Vec::new();
        for item in &self.id_list {
            item.write(&mut buffer)?;
        }
        write_u16(data, buffer.len() as u16)?;
        data.write_all(&buffer)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum IdEntry {
    Root(RootLocationType),
    Drive(char),
    Folder(IdEntryData),
    File(IdEntryData),
}

#[derive(Debug)]
pub struct IdEntryData {
    pub filesize: u32,
    pub modified: NaiveDateTime,
    pub short_name: String,
    pub created: Option<NaiveDateTime>,
    pub accessed: Option<NaiveDateTime>,
    pub full_name: Option<String>,
    pub localized_name: Option<String>,
}

impl IdEntryData {
    pub fn write(&self, data: &mut impl Write) -> Result<(), LnkWriteError> {
        write_u32(data, self.filesize)?;
        write_dos_datetime(data, self.modified);
        

        todo!()
    }
}

impl IdEntry {
    fn parse(data: &mut impl Read) -> Result<Self, IdListParseError> {
        let first_type_byte = read_u8(data)?;

        let entry_type = match first_type_byte {
            0x1f => EntryType::RootGuid,
            0x2f => EntryType::Drive,
            _ => {
                let second_type_byte = read_u8(data)?;
                let type_id = u16::from_le_bytes([first_type_byte, second_type_byte]);
                EntryType::from_type_id(type_id)
                    .ok_or_else(|| IdListParseError::InvalidEntryType(type_id))?
            }
        };

        match entry_type {
            EntryType::RootGuid => {
                let _root_index = read_u8(data)?;
                let guid = read_guid(data)?;

                let guid = RootLocationType::from_guid(guid)
                    .ok_or_else(|| IdListParseError::InvalidRootType)?;

                Ok(Self::Root(guid))
            }

            EntryType::Drive => {
                let letter = read_u8(data)? as char;

                if !letter.is_ascii_uppercase() {
                    return Err(IdListParseError::InvalidDriveEntry);
                }

                if read_u8(data)? != 0x3a {
                    return Err(IdListParseError::InvalidDriveEntry);
                }
                if read_u8(data)? != 0x5c {
                    return Err(IdListParseError::InvalidDriveEntry);
                }
                let mut junk_data = [0u8; 19];
                data.read_exact(&mut junk_data)?;

                Ok(Self::Drive(letter))
            }

            EntryType::File
            | EntryType::FileUnicode
            | EntryType::Folder
            | EntryType::FolderUnicode => {
                let filesize = read_u32(data)?;
                let modified = read_dos_datetime(data)?;
                let _file_attributes_l = read_u16(data)?;
                let short_name = if entry_type.is_unicode() {
                    read_c_utf16(data)?
                } else {
                    read_c_utf8(data, true)?
                };
                let mut entry_data = IdEntryData {
                    filesize,
                    modified,
                    short_name,
                    accessed: None,
                    created: None,
                    full_name: None,
                    localized_name: None,
                };

                let extra_size = read_u16(data)?;
                let extra_version = read_u16(data)?;
                let extra_signature = read_u32(data)?;
                if extra_signature == 0xbeef0004 {
                    let mut data = data.take(extra_size as u64);
                    let data = &mut data;
                    entry_data.created = Some(read_dos_datetime(data)?);
                    entry_data.accessed = Some(read_dos_datetime(data)?);
                    let _offset_unicode = read_u16(data)?;
                    if extra_version >= 7 {
                        let _offset_ansi = read_u16(data)?;
                        let _file_reference = read_u64(data)?;
                        let _unknown_2 = read_u64(data)?;
                    }
                    let long_string_size = if extra_version >= 3 {
                        read_u16(data)?
                    } else {
                        0
                    };
                    if extra_version >= 9 {
                        let _unknown_4 = read_u32(data)?;
                    }
                    if extra_version >= 8 {
                        let _unknown_5 = read_u32(data)?;
                    }
                    if extra_version >= 3 {
                        entry_data.full_name = Some(read_c_utf16(data)?);
                        if long_string_size > 0 {
                            let localized = if extra_version >= 7 {
                                read_c_utf16(data)?
                            } else {
                                read_c_utf8(data, false)?
                            };
                            entry_data.localized_name = Some(localized)
                        }
                        let _version_offset = read_u16(data)?;
                    }

                    let mut left = Vec::new();
                    let read = data.read_to_end(&mut left)?;
                    if read != 0 {
                        return Err(IdListParseError::BytesLeft);
                    }
                }

                match entry_type {
                    EntryType::File | EntryType::FileUnicode => Ok(Self::File(entry_data)),
                    EntryType::Folder | EntryType::FolderUnicode => Ok(Self::Folder(entry_data)),
                    _ => Err(IdListParseError::UnsupportedEntryType),
                }
            }
            _ => Err(IdListParseError::UnsupportedEntryType),
        }
    }

    pub fn write(&self, data: &mut impl Write) -> Result<(), LnkWriteError> {
        match self {
            Self::Root(root_type) => {
                write_u8(data, 0x1f)?;
                write_u8(data, 0x50)?;
                let guid = root_type.guid();
                guid.write(data)?;
            }
            Self::Drive(letter) => {
                write_u8(data, 0x2f)?;
                let mut dst = [0u8; 4];
                letter.encode_utf8(&mut dst);
                write_u8(data, dst[0])?;
                write_u8(data, 0x3a)?;
                write_u8(data, 0x5c)?;
                let junk_data = [0u8; 19];
                data.write_all(&junk_data)?;
            }
            Self::Folder(id_entry) => {
                write_u16(data, EntryType::FolderUnicode.to_type_id())?;
                id_entry.write(data)?;
            }
            Self::File(id_entry) => {
                write_u16(data, EntryType::FileUnicode.to_type_id())?;
                id_entry.write(data)?;
            }
        }
        todo!()
    }
}

enum EntryType {
    KnownFolder,
    Folder,
    File,
    FolderUnicode,
    FileUnicode,
    KnownRootFolder,
    RootFolder,
    RootGuid,
    Drive,
    Uri,
    ControlPanel,
}

impl EntryType {
    fn from_type_id(type_id: u16) -> Option<Self> {
        match type_id {
            0x00 => Some(Self::KnownFolder),
            0x31 => Some(Self::Folder),
            0x32 => Some(Self::File),
            0x35 => Some(Self::FolderUnicode),
            0x36 => Some(Self::FileUnicode),
            0x802e => Some(Self::KnownRootFolder),
            0x1f => Some(Self::RootFolder),
            0x61 => Some(Self::Uri),
            0x71 => Some(Self::ControlPanel),
            _ => None,
        }
    }

    fn is_unicode(&self) -> bool {
        match self {
            Self::FileUnicode => true,
            Self::FolderUnicode => true,
            _ => false,
        }
    }

    fn to_type_id(&self) -> u16 {
        match self {
            Self::KnownFolder => 0x00,
            Self::Folder => 0x31,
            Self::File => 0x32,
            Self::FolderUnicode => 0x35,
            Self::FileUnicode => 0x36,
            Self::KnownRootFolder => 0x802e,
            Self::RootFolder => 0x1f,
            Self::RootGuid => 0x61,
            Self::Drive => 0x71,
            Self::Uri => 0x71,
            Self::ControlPanel => 0x71,
        }
    }
}

#[derive(Debug)]
pub enum RootLocationType {
    MyComputer,
    MyDocuments,
    NetworkShare,
    NetworkServer,
    NetworkPlaces,
    NetworkDomain,
    Internet,
    RecycleBin,
    ControlPanel,
    User,
    UwpApps,
}

impl RootLocationType {
    fn from_guid(guid: Guid) -> Option<Self> {
        let text = guid.to_string();
        Self::from_text_guid(text.as_bytes())
    }

    fn from_text_guid(guid: &[u8]) -> Option<Self> {
        match guid {
            b"{20D04FE0-3AEA-1069-A2D8-08002B30309D}" => Some(Self::MyComputer),
            b"{450D8FBA-AD25-11D0-98A8-0800361B1103}" => Some(Self::MyDocuments),
            b"{54a754c0-4bf1-11d1-83ee-00a0c90dc849}" => Some(Self::NetworkShare),
            b"{c0542a90-4bf0-11d1-83ee-00a0c90dc849}" => Some(Self::NetworkServer),
            b"{208D2C60-3AEA-1069-A2D7-08002B30309D}" => Some(Self::NetworkPlaces),
            b"{46e06680-4bf0-11d1-83ee-00a0c90dc849}" => Some(Self::NetworkDomain),
            b"{871C5380-42A0-1069-A2EA-08002B30309D}" => Some(Self::Internet),
            b"{645FF040-5081-101B-9F08-00AA002F954E}" => Some(Self::RecycleBin),
            b"{21EC2020-3AEA-1069-A2DD-08002B30309D}" => Some(Self::ControlPanel),
            b"{59031A47-3F72-44A7-89C5-5595FE6B30EE}" => Some(Self::User),
            b"{4234D49B-0245-4DF3-B780-3893943456E1}" => Some(Self::UwpApps),
            _ => None,
        }
    }

    fn str(&self) -> &str {
        match self {
            Self::MyComputer => "{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
            Self::MyDocuments => "{450D8FBA-AD25-11D0-98A8-0800361B1103}",
            Self::NetworkShare => "{54a754c0-4bf1-11d1-83ee-00a0c90dc849}",
            Self::NetworkServer => "{c0542a90-4bf0-11d1-83ee-00a0c90dc849}",
            Self::NetworkPlaces => "{208D2C60-3AEA-1069-A2D7-08002B30309D}",
            Self::NetworkDomain => "{46e06680-4bf0-11d1-83ee-00a0c90dc849}",
            Self::Internet => "{871C5380-42A0-1069-A2EA-08002B30309D}",
            Self::RecycleBin => "{645FF040-5081-101B-9F08-00AA002F954E}",
            Self::ControlPanel => "{21EC2020-3AEA-1069-A2DD-08002B30309D}",
            Self::User => "{59031A47-3F72-44A7-89C5-5595FE6B30EE}",
            Self::UwpApps => "{4234D49B-0245-4DF3-B780-3893943456E1}",
        }
    }

    fn guid(&self) -> Guid {
        let text = self.str();
        Guid::from_str(text).unwrap()
    }
}
