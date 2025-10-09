use bitflags::bitflags;
use chrono::NaiveDateTime;
use std::io::{self, Read, Write};

use crate::lnk::{
    block_data::BlockData,
    helpers::{
        StringReadError, WindowsDateTimeError, read_i32, read_sized_string, read_u16, read_u32,
        read_windows_datetime, write_c_utf16, write_i32, write_sized_utf16, write_u16, write_u32,
        write_windows_datetime,
    },
    id_list::IdList,
    link_info::LinkInfo,
};

mod block_data;
mod helpers;
mod id_list;
mod link_info;

#[derive(Debug, thiserror::Error)]
pub enum LnkParseError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Error reading Windows DateTime: {0}")]
    WindowsDateTimeError(#[from] WindowsDateTimeError),
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
    #[error("error while parsing link info: {0}")]
    LinkInfoError(#[from] link_info::LinkInfoParseError),
    #[error("error reading string: {0}")]
    StringReadError(#[from] StringReadError),
    #[error("error reading block data: {0}")]
    BlockDataError(#[from] block_data::BlockDataParseError),
    #[error("unparsed data left: {0:?}")]
    RemainingData(Vec<u8>),
}

#[derive(Debug)]
pub struct Lnk {
    pub link_flags: LinkFlags,
    pub file_flags: FileAttributeFlags,
    pub creation_time: NaiveDateTime,
    pub access_time: NaiveDateTime,
    pub modification_time: NaiveDateTime,
    pub file_size_lower_bytes: u32,
    pub icon_index: i32,
    pub show_command: ShowCommand,
    pub id_list: Option<IdList>,
    pub link_info: Option<LinkInfo>,
    pub name: Option<String>,
    pub relative_path: Option<String>,
    pub working_dir: Option<String>,
    pub arguments: Option<String>,
    pub icon_location: Option<String>,
    pub block_data: BlockData,
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

        let link_flags = read_u32(data)?;
        let link_flags = LinkFlags::from_bits(link_flags)
            .ok_or_else(|| LnkParseError::InvalidLinkFlags(link_flags))?;

        let file_flags = read_u32(data)?;
        let file_flags = FileAttributeFlags::from_bits(file_flags)
            .ok_or_else(|| LnkParseError::InvalidFileFlags(file_flags))?;

        let creation_time = read_windows_datetime(data)?;
        let access_time = read_windows_datetime(data)?;
        let modification_time = read_windows_datetime(data)?;
        let file_size_lower_bytes = read_u32(data)?;
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

        let link_info = if link_flags.contains(LinkFlags::HAS_LINK_INFO)
            && !link_flags.contains(LinkFlags::FORCE_NO_LINK_INFO)
        {
            Some(LinkInfo::parse(data)?)
        } else {
            None
        };

        let utf16 = link_flags.contains(LinkFlags::IS_UNICODE);

        let name = if link_flags.contains(LinkFlags::HAS_NAME) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        let relative_path = if link_flags.contains(LinkFlags::HAS_RELATIVE_PATH) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        let working_dir = if link_flags.contains(LinkFlags::HAS_WORKING_DIR) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        let arguments = if link_flags.contains(LinkFlags::HAS_ARGUMENTS) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        let icon_location = if link_flags.contains(LinkFlags::HAS_ICON_LOCATION) {
            Some(read_sized_string(data, utf16)?)
        } else {
            None
        };

        let block_data = BlockData::parse(data)?;

        let lnk = Self {
            link_flags,
            file_flags,
            creation_time,
            access_time,
            modification_time,
            file_size_lower_bytes,
            icon_index,
            show_command,
            id_list,
            link_info,
            name,
            relative_path,
            working_dir,
            arguments,
            icon_location,
            block_data,
        };

        let mut remaining_data = Vec::new();
        if data.read_to_end(&mut remaining_data)? > 0 {
            return Err(LnkParseError::RemainingData(remaining_data));
        }

        Ok(lnk)
    }

    pub fn write(&self, data: &mut impl Write) -> Result<(), LnkWriteError> {
        data.write_all(SIGNATURE)?;
        data.write_all(GUID)?;

        let mut link_flags = self.link_flags.clone();
        link_flags.insert(LinkFlags::IS_UNICODE);
        link_flags.set(LinkFlags::HAS_LINK_TARGET_ID_LIST, self.id_list.is_some());
        link_flags.set(LinkFlags::HAS_NAME, self.name.is_some());
        link_flags.set(LinkFlags::HAS_RELATIVE_PATH, self.relative_path.is_some());
        link_flags.set(LinkFlags::HAS_WORKING_DIR, self.working_dir.is_some());
        link_flags.set(LinkFlags::HAS_ARGUMENTS, self.arguments.is_some());
        link_flags.set(LinkFlags::HAS_ICON_LOCATION, self.icon_location.is_some());
        link_flags.set(
            LinkFlags::HAS_EXP_ICON,
            self.block_data.icon_environment.is_some(),
        );

        write_u32(data, link_flags.bits())?;
        write_u32(data, self.file_flags.bits())?;

        write_windows_datetime(data, self.creation_time)?;
        write_windows_datetime(data, self.access_time)?;
        write_windows_datetime(data, self.modification_time)?;

        write_u32(data, self.file_size_lower_bytes)?;
        write_i32(data, self.icon_index)?;

        write_u32(data, self.show_command.to_u32())?;

        // Hotkey
        write_u16(data, 0)?;
        // Reserved 1
        write_u16(data, 0)?;
        // Reserved 2
        write_u16(data, 0)?;
        // Reserved 3
        write_u16(data, 0)?;

        if let Some(id_list) = &self.id_list {
            id_list.write(data)?;
        }

        if let Some(link_info) = &self.link_info {
            link_info.write(data)?;
        }

        if let Some(name) = &self.name {
            write_sized_utf16(data, name)?;
        }

        if let Some(relative_path) = &self.relative_path {
            write_sized_utf16(data, relative_path)?;
        }

        if let Some(working_dir) = &self.working_dir {
            write_sized_utf16(data, working_dir)?;
        }

        if let Some(arguments) = &self.arguments {
            write_sized_utf16(data, arguments)?;
        }

        if let Some(icon_location) = &self.icon_location {
            write_sized_utf16(data, icon_location)?;
        }

        self.block_data.write(data)?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LnkWriteError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

const SIGNATURE: &[u8] = b"L\x00\x00\x00";
const GUID: &[u8] = b"\x01\x14\x02\x00\x00\x00\x00\x00\xc0\x00\x00\x00\x00\x00\x00F";

#[derive(Debug, Clone)]
pub enum ShowCommand {
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

    fn to_u32(&self) -> u32 {
        match self {
            ShowCommand::Normal => 1,
            ShowCommand::GrabFocus => 3,
            ShowCommand::SkipFocus => 7,
        }
    }
}

bitflags! {
    /// The LinkFlags structure defines bits that specify which shell link structures are present in the file
    /// format after the ShellLinkHeader structure (section 2.1).
    #[derive(Debug, Clone)]
    pub struct LinkFlags: u32 {
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
    pub struct FileAttributeFlags: u32 {
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
