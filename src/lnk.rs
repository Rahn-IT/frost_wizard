use bitflags::bitflags;
use byteorder::{BE, LE, ReadBytesExt};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use std::{
    collections::VecDeque,
    io::{self, Read},
    path::PathBuf,
};

use crate::lnk::id_list::IdList;

mod id_list;

#[derive(Debug, thiserror::Error)]
pub enum LnkParseError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid Windows DateTime: {0}")]
    InvalidWindowsDateTime(u64),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid GUID")]
    InvalidGuid,
    #[error("Invalid link flags {0:032b}")]
    InvalidLinkFlags(u32),
    #[error("Invalid file flags {0:032b}")]
    InvalidFileFlags(u32),
    #[error("Invalid show command {0}")]
    InvalidShowCommand(u32),
    #[error("error while parsing id list: {0}")]
    IdListError(#[from] id_list::IdListParseError),
    #[error("error reading string: {0}")]
    StringReadError(#[from] StringReadError),
}

#[derive(Debug)]
pub struct Lnk {
    link_flags: LinkFlags,
    file_flags: FileAttributeFlags,
    creation_time: NaiveDateTime,
    access_time: NaiveDateTime,
    modification_time: NaiveDateTime,
    file_size: u32,
    icon_index: i32,
    show_command: ShowCommand,
    id_list: Option<IdList>,
    name: Option<String>,
    relative_path: Option<String>,
    working_dir: Option<String>,
    arguments: Option<String>,
    icon_location: Option<String>,
}

impl Lnk {
    pub fn parse(data: &mut impl Read) -> Result<Self, LnkParseError> {
        let mut signature = [0u8; 4];
        data.read_exact(&mut signature)?;
        if signature != *SIGNATURE {
            return Err(LnkParseError::InvalidSignature);
        }

        let mut guid = [0u8; 16];
        data.read_exact(&mut guid)?;
        if guid != *GUID {
            return Err(LnkParseError::InvalidGuid);
        }

        let link_flags = read_flags(data)?;
        println!("link_flags: {link_flags:032b}");
        let link_flags = LinkFlags::from_bits(link_flags)
            .ok_or_else(|| LnkParseError::InvalidLinkFlags(link_flags))?;

        println!("link_flags: {link_flags:?}");

        let file_flags = read_flags(data)?;
        let file_flags = FileAttributeFlags::from_bits(file_flags)
            .ok_or_else(|| LnkParseError::InvalidFileFlags(file_flags))?;

        let creation_time = read_windows_datetime(data)?;
        let access_time = read_windows_datetime(data)?;
        let modification_time = read_windows_datetime(data)?;
        let file_size = read_u32(data)?;
        let icon_index = read_i32(data)?;

        let show_command = ShowCommand::from_u32(read_u32(data)?)?;

        let _hotkey = read_u16(data)?;
        let _reserved1 = read_u16(data)?;
        let _reserved2 = read_u32(data)?;
        let _reserved3 = read_u32(data)?;

        let id_list = if link_flags.contains(LinkFlags::HAS_LINK_TARGET_ID_LIST) {
            Some(IdList::parse(data)?)
        } else {
            None
        };

        if link_flags.contains(LinkFlags::HAS_LINK_INFO) {
            todo!()
        }

        let utf16 = link_flags.contains(LinkFlags::IS_UNICODE);
        println!("utf16: {utf16}");

        let name = if link_flags.contains(LinkFlags::HAS_NAME) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        println!("name: {name:?}");

        let relative_path = if link_flags.contains(LinkFlags::HAS_RELATIVE_PATH) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };
        println!("relative_path: {relative_path:?}");

        let working_dir = if link_flags.contains(LinkFlags::HAS_WORKING_DIR) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };
        println!("working_dir: {working_dir:?}");

        let arguments = if link_flags.contains(LinkFlags::HAS_ARGUMENTS) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };
        println!("arguments: {arguments:?}");

        let icon_location = if link_flags.contains(LinkFlags::HAS_ICON_LOCATION) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };
        println!("icon_location: {icon_location:?}");

        Ok(Self {
            link_flags,
            file_flags,
            creation_time,
            access_time,
            modification_time,
            file_size,
            icon_index,
            show_command,
            id_list,
            name,
            relative_path,
            working_dir,
            arguments,
            icon_location,
        })
    }
}

const SIGNATURE: &[u8] = b"L\x00\x00\x00";
const GUID: &[u8] = b"\x01\x14\x02\x00\x00\x00\x00\x00\xc0\x00\x00\x00\x00\x00\x00F";
const LINK_INFO_HEADER_DEFAULT: u8 = 0x1C;
const LINK_INFO_HEADER_OPTIONAL: u8 = 0x24;

fn read_byte(data: &mut impl Read) -> io::Result<u8> {
    data.read_u8()
}

#[must_use]
fn read_u16(data: &mut impl Read) -> io::Result<u16> {
    data.read_u16::<LE>()
}

#[must_use]
fn read_u32(data: &mut impl Read) -> io::Result<u32> {
    data.read_u32::<LE>()
}

#[must_use]
fn read_i32(data: &mut impl Read) -> io::Result<i32> {
    data.read_i32::<LE>()
}

#[must_use]
fn read_flags(data: &mut impl Read) -> io::Result<u32> {
    data.read_u32::<LE>()
}

#[must_use]
fn read_u64(data: &mut impl Read) -> io::Result<u64> {
    data.read_u64::<LE>()
}

const WINDOWS_EPOCH: u64 = 11644473600;

#[must_use]
fn read_windows_datetime(data: &mut impl Read) -> Result<NaiveDateTime, LnkParseError> {
    let windows_timestamp = read_u64(data)?;
    let unix_timestamp = windows_timestamp / 10_000_000 - WINDOWS_EPOCH;

    let datetime = DateTime::from_timestamp(unix_timestamp as i64, 0)
        .ok_or_else(|| LnkParseError::InvalidWindowsDateTime(windows_timestamp))?;

    Ok(datetime.naive_utc())
}

#[derive(Debug, thiserror::Error)]
pub enum StringReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("UTF-16 error: {0}")]
    Utf16Error(#[from] std::string::FromUtf16Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

fn read_sized_string(data: &mut impl Read, utf16: bool) -> Result<String, StringReadError> {
    if utf16 {
        read_sized_utf16(data)
    } else {
        read_sized_utf8(data)
    }
}

#[must_use]
fn read_sized_utf16(data: &mut impl Read) -> Result<String, StringReadError> {
    let size = read_u16(data)? as usize;
    let mut raw_string = vec![0u8; size * 2];
    data.read_exact(&mut raw_string)?;
    let mut iter = raw_string.into_iter();
    let mut utf16 = Vec::with_capacity(size);
    while let Some((byte1, byte2)) = iter.next().zip(iter.next()) {
        let short = u16::from_le_bytes([byte1, byte2]);
        utf16.push(short);
    }

    Ok(String::from_utf16(&utf16)?)
}

#[must_use]
fn read_c_utf16(data: &mut impl Read) -> Result<String, StringReadError> {
    let mut encoded_string = Vec::new();
    loop {
        let short = read_u16(data)?;
        if short == 0 {
            break;
        }
        encoded_string.push(short);
    }

    let decoded_string = String::from_utf16(&encoded_string)?;
    Ok(decoded_string)
}

#[must_use]
fn read_sized_utf8(data: &mut impl Read) -> Result<String, StringReadError> {
    let size = read_u16(data)?;
    let mut raw_string = vec![0u8; size as usize];
    data.read_exact(&mut raw_string)?;
    Ok(String::from_utf8(raw_string)?)
}

#[must_use]
fn read_c_utf8(data: &mut impl Read, padding: bool) -> Result<String, StringReadError> {
    let mut encoded_string = Vec::new();
    loop {
        let byte = read_byte(data)?;
        if byte == 0 {
            break;
        }
        encoded_string.push(byte);
    }

    if padding && encoded_string.len() % 2 == 0 {
        let _padding = read_byte(data)?;
    }

    let decoded_string = String::from_utf8(encoded_string)?;
    Ok(decoded_string)
}

fn get_bits(short: u16, start: u8, length: u8) -> u16 {
    let mask = (1 << length) - 1;
    let shifted = short >> start;
    let result = shifted & mask;
    result
}

#[derive(Debug, thiserror::Error)]
pub enum DosDateTimeReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid DOS date: {0}-{1}-{2}")]
    InvalidDosDate(u16, u16, u16),
    #[error("Invalid DOS time: {0}:{1}:{2}")]
    InvalidDosTime(u16, u16, u16),
}

fn read_dos_datetime(data: &mut impl Read) -> Result<NaiveDateTime, DosDateTimeReadError> {
    let date = read_u16(data)?;
    let time = read_u16(data)?;
    let year = get_bits(date, 9, 7) + 1980;
    let month = get_bits(date, 5, 4).max(1);
    let day = get_bits(date, 0, 5).max(1);
    let hour = get_bits(time, 11, 5);
    let minute = get_bits(time, 5, 6);
    let second = get_bits(time, 0, 5);

    let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
        .ok_or_else(|| DosDateTimeReadError::InvalidDosDate(year, month, day))?;

    let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
        .ok_or_else(|| DosDateTimeReadError::InvalidDosTime(hour, minute, second))?;

    Ok(NaiveDateTime::new(date, time))
}

#[derive(Debug, Clone)]
enum ShowCommand {
    Normal = 1,
    GrabFocus = 3,
    SkipFocus = 7,
}

impl ShowCommand {
    fn from_u32(value: u32) -> Result<Self, LnkParseError> {
        match value {
            1 => Ok(ShowCommand::Normal),
            3 => Ok(ShowCommand::GrabFocus),
            7 => Ok(ShowCommand::SkipFocus),
            _ => Err(LnkParseError::InvalidShowCommand(value)),
        }
    }
}

bitflags! {
    /// The LinkFlags structure defines bits that specify which shell link structures are present in the file
    /// format after the ShellLinkHeader structure (section 2.1).
    #[derive(Debug, Clone)]
    struct LinkFlags: u32 {
        /// The shell link is saved with an item ID list (IDList). If this bit is set, a
        /// LinkTargetIDList structure (section 2.2) MUST follow the ShellLinkHeader.
        /// If this bit is not set, this structure MUST NOT be present.
        const HAS_LINK_TARGET_ID_LIST           = 0b0000_0000_0000_0000_0000_0000_0000_0001;

        /// The shell link is saved with link information. If this bit is set, a LinkInfo
        /// structure (section 2.3) MUST be present. If this bit is not set, this structure
        /// MUST NOT be present.
        const HAS_LINK_INFO                     = 0b0000_0000_0000_0000_0000_0000_0000_0010;

        ///The shell link is saved with a name string. If this bit is set, a
        ///NAME_STRING StringData structure (section 2.4) MUST be present. If
        ///this bit is not set, this structure MUST NOT be present.
        const HAS_NAME                          = 0b0000_0000_0000_0000_0000_0000_0000_0100;

        /// The shell link is saved with a relative path string. If this bit is set, a
        /// RELATIVE_PATH StringData structure (section 2.4) MUST be present. If
        /// this bit is not set, this structure MUST NOT be present.
        const HAS_RELATIVE_PATH                 = 0b0000_0000_0000_0000_0000_0000_0000_1000;

        /// The shell link is saved with a working directory string. If this bit is set, a
        /// WORKING_DIR StringData structure (section 2.4) MUST be present. If
        /// this bit is not set, this structure MUST NOT be present.
        const HAS_WORKING_DIR                   = 0b0000_0000_0000_0000_0000_0000_0001_0000;

        /// The shell link is saved with command line arguments. If this bit is set, a
        /// COMMAND_LINE_ARGUMENTS StringData structure (section 2.4) MUST
        /// be present. If this bit is not set, this structure MUST NOT be present.
        const HAS_ARGUMENTS                     = 0b0000_0000_0000_0000_0000_0000_0010_0000;

        /// The shell link is saved with an icon location string. If this bit is set, an
        /// ICON_LOCATION StringData structure (section 2.4) MUST be present. If
        /// this bit is not set, this structure MUST NOT be present.
        const HAS_ICON_LOCATION                 = 0b0000_0000_0000_0000_0000_0000_0100_0000;

        /// The shell link contains Unicode encoded strings. This bit SHOULD be set. If
        /// this bit is set, the StringData section contains Unicode-encoded strings;
        /// otherwise, it contains strings that are encoded using the system default
        /// code page.
        const IS_UNICODE                        = 0b0000_0000_0000_0000_0000_0000_1000_0000;

        /// The LinkInfo structure (section 2.3) is ignored.
        const FORCE_NO_LINK_INFO                = 0b0000_0000_0000_0000_0000_0001_0000_0000;

        /// The shell link is saved with an
        /// EnvironmentVariableDataBlock (section 2.5.4).
        const HAS_EXP_STRING                    = 0b0000_0000_0000_0000_0000_0010_0000_0000;

        /// The target is run in a separate virtual machine when launching a link
        /// target that is a 16-bit application.
        const RUN_IN_SEPARATE_PROCESS           = 0b0000_0000_0000_0000_0000_0100_0000_0000;

        /// A bit that is undefined and MUST be ignored.
        const UNUSED_1                          = 0b0000_0000_0000_0000_0000_1000_0000_0000;

        /// The shell link is saved with a DarwinDataBlock (section 2.5.3).
        const HAS_DARWIN_ID                     = 0b0000_0000_0000_0000_0001_0000_0000_0000;

        /// The application is run as a different user when the target of the shell link is
        /// activated.
        const RUN_AS_USER                       = 0b0000_0000_0000_0000_0010_0000_0000_0000;

        /// The shell link is saved with an IconEnvironmentDataBlock (section 2.5.5).
        const HAS_EXP_ICON                      = 0b0000_0000_0000_0000_0100_0000_0000_0000;

        /// The file system location is represented in the shell namespace when the
        /// path to an item is parsed into an IDList.
        const NO_PID_I_ALIAS                    = 0b0000_0000_0000_0000_1000_0000_0000_0000;

        /// A bit that is undefined and MUST be ignored.
        const UNUSED_2                          = 0b0000_0000_0000_0001_0000_0000_0000_0000;

        /// The shell link is saved with a ShimDataBlock (section 2.5.8).
        const RUN_WITH_SHIM_LAYER               = 0b0000_0000_0000_0010_0000_0000_0000_0000;

        /// The TrackerDataBlock (section 2.5.10) is ignored.
        const FORCE_NO_LINK_TRACK               = 0b0000_0000_0000_0100_0000_0000_0000_0000;

        /// The shell link attempts to collect target properties and store them in the
        /// PropertyStoreDataBlock (section 2.5.7) when the link target is set.
        const ENABLE_TARGET_METADATA            = 0b0000_0000_0000_1000_0000_0000_0000_0000;

        /// The EnvironmentVariableDataBlock is ignored.
        const DISABLE_LINK_PATH_TRACKING        = 0b0000_0000_0001_0000_0000_0000_0000_0000;

        /// The SpecialFolderDataBlock (section 2.5.9) and the
        /// KnownFolderDataBlock (section 2.5.6) are ignored when loading the shell
        /// link. If this bit is set, these extra data blocks SHOULD NOT be saved when
        /// saving the shell link.
        const DISABLE_KNOWN_FOLDER_TRACKING     = 0b0000_0000_0010_0000_0000_0000_0000_0000;

        /// If the link has a KnownFolderDataBlock (section 2.5.6), the unaliased form
        /// of the known folder IDList SHOULD be used when translating the target
        /// IDList at the time that the link is loaded.
        const DISABLE_KNOWN_FOLDER_ALIAS        = 0b0000_0000_0100_0000_0000_0000_0000_0000;

        /// Creating a link that references another link is enabled. Otherwise,
        /// specifying a link as the target IDList SHOULD NOT be allowed.
        const ALLOW_LINK_TO_LINK                = 0b0000_0000_1000_0000_0000_0000_0000_0000;

        /// When saving a link for which the target IDList is under a known folder,
        /// either the unaliased form of that known folder or the target IDList SHOULD
        /// be used.
        const UNALIAS_ON_SAVE                   = 0b0000_0001_0000_0000_0000_0000_0000_0000;

        /// The target IDList SHOULD NOT be stored; instead, the path specified in the
        /// EnvironmentVariableDataBlock (section 2.5.4) SHOULD be used to refer to
        /// the target.
        const PREFER_ENVIRONMENT_PATH           = 0b0000_0010_0000_0000_0000_0000_0000_0000;

        /// When the target is a UNC name that refers to a location on a local
        /// machine, the local path IDList in the
        /// PropertyStoreDataBlock (section 2.5.7) SHOULD be stored, so it can be
        /// used when the link is loaded on the local machine.
        const KEEP_LOCAL_ID_LIST_FOR_UNC_TARGET = 0b0000_0100_0000_0000_0000_0000_0000_0000;
    }
}

bitflags! {
    /// The FileAttributesFlags structure defines bits that specify the file attributes of the link target, if the
    /// target is a file system item. File attributes can be used if the link target is not available, or if accessing
    /// the target would be inefficient. It is possible for the target items attributes to be out of sync with this
    /// value.
    #[derive(Debug, Clone)]
    struct FileAttributeFlags: u32 {
        /// The file or directory is read-only. For a file, if this bit is set, applications can read the file but cannot write to it or delete it. For a directory, if this bit is set, applications cannot delete the directory.
        const FILE_ATTRIBUTE_READONLY               = 0b0000_0000_0000_0000_0000_0000_0000_0001;

        /// The file or directory is hidden. If this bit is set, the file or folder is not included in an ordinary directory listing.
        const FILE_ATTRIBUTE_HIDDEN                 = 0b0000_0000_0000_0000_0000_0000_0000_0010;

        /// The file or directory is part of the operating system or is used exclusively by the operating system.
        const FILE_ATTRIBUTE_SYSTEM                 = 0b0000_0000_0000_0000_0000_0000_0000_0100;

        /// A bit that MUST be zero.
        const RESERVED_1                            = 0b0000_0000_0000_0000_0000_0000_0000_1000;

        /// The link target is a directory instead of a file.
        const FILE_ATTRIBUTE_DIRECTORY              = 0b0000_0000_0000_0000_0000_0000_0001_0000;

        /// The file or directory is an archive file. Applications use this flag to mark files for backup or removal.
        const FILE_ATTRIBUTE_ARCHIVE                = 0b0000_0000_0000_0000_0000_0000_0010_0000;

        /// A bit that MUST be zero.
        const RESERVED_2                            = 0b0000_0000_0000_0000_0000_0000_0100_0000;

        /// The file or directory has no other flags set. If this bit is 1, all other bits in this structure MUST be clear.
        const FILE_ATTRIBUTE_NORMAL                 = 0b0000_0000_0000_0000_0000_0000_1000_0000;

        /// The file is being used for temporary storage.
        const FILE_ATTRIBUTE_TEMPORARY              = 0b0000_0000_0000_0000_0000_0001_0000_0000;

        /// The file is a sparse file.
        const FILE_ATTRIBUTE_SPARCE_FILE            = 0b0000_0000_0000_0000_0000_0010_0000_0000;

        /// The file or directory has an associated reparse point.
        const FILE_ATTRIBUTE_REPARSE_POINT          = 0b0000_0000_0000_0000_0000_0100_0000_0000;

        /// The file or directory is compressed. For a file, this means that all data in the file is compressed. For a directory, this means that compression is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_COMPRESSED             = 0b0000_0000_0000_0000_0000_1000_0000_0000;

        /// The data of the file is not immediately available.
        const FILE_ATTRIBUTE_OFFLINE                = 0b0000_0000_0000_0000_0001_0000_0000_0000;

        /// The contents of the file need to be indexed.
        const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED    = 0b0000_0000_0000_0000_0010_0000_0000_0000;

        /// The file or directory is encrypted. For a file, this means that all data in the file is encrypted. For a directory, this means that encryption is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_ENCRYPTED              = 0b0000_0000_0000_0000_0100_0000_0000_0000;

    }
}
