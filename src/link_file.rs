use bitflags::bitflags;
use byteorder::{LE, WriteBytesExt};
use chrono::{DateTime, Utc};
use std::{
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// A lot of this code is derived from or stolen from parselnk-rs:
/// https://github.com/rustysec/parselnk-rs/tree/master

pub fn write_link(writer: &mut impl Write, target: &Path) -> Result<(), io::Error> {
    write_header(writer)?;
    write_target_id_list(writer, target)?;

    Ok(())
}

#[derive(Debug, Clone)]
enum ShowCommand {
    Normal = 1,
    GrabFocus = 3,
    SkipFocus = 7,
}

const WINDOWS_EPOCH: u64 = 116444736000000000;

/// Header according to:
/// https://winprotocoldoc.z19.web.core.windows.net/MS-SHLLINK/%5bMS-SHLLINK%5d.pdf
fn write_header(writer: &mut impl Write) -> Result<(), io::Error> {
    // HeaderSize
    writer.write_u32::<LE>(0x4c)?;
    // LinkCLSID
    writer.write_u32::<LE>(0x21401)?;
    writer.write_u32::<LE>(0x0)?;
    writer.write_u32::<LE>(0xc0)?;
    writer.write_u32::<LE>(0x46000000)?;
    // LinkFlags
    writer.write_u32::<LE>(
        LinkFlags::HAS_LINK_TARGET_ID_LIST
            .union(LinkFlags::IS_UNICODE)
            .bits(),
    )?;
    // FileAttributes
    writer.write_u32::<LE>(FileAttributeFlags::FILE_ATTRIBUTE_NORMAL.bits())?;

    // Yeah, so this is supposed to be data from the link target? What if the link target changes???
    // let windows_filetime = (SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .expect("Time went backwards")
    //     .as_nanos()
    //     / 100) as u64
    //     + WINDOWS_EPOCH;

    // CreationTime
    writer.write_u64::<LE>(0)?;
    // AccessTime
    writer.write_u64::<LE>(0)?;
    // ModifiedTime
    writer.write_u64::<LE>(0)?;

    let file_size = 0;

    // FileSize
    writer.write_u32::<LE>(file_size)?;
    // IconIndex
    writer.write_u32::<LE>(0)?;

    // ShowCommand
    writer.write_u32::<LE>(ShowCommand::Normal as u32)?;

    // HotKey
    writer.write_u16::<LE>(0)?;

    // Reserved 1
    writer.write_u16::<LE>(0)?;

    // Reserved 2
    writer.write_u32::<LE>(0)?;

    // Reserved 3
    writer.write_u32::<LE>(0)?;

    Ok(())
}

fn write_target_id_list(writer: &mut impl Write, target: &Path) -> Result<(), io::Error> {
    let mut buffer = Vec::new();

    // For local files, the IDList should start with a root folder (e.g., My Computer)
    // Root folder (My Computer)
    buffer.write_u16::<LE>(0x001F)?; // Size of this item (31 bytes)
    buffer.write_u8(0x20)?; // Type: PT_GUID (0x20)
    buffer.write_u8(0x00)?; // Flags
    buffer.write_all(&[
        0x20, 0xD0, 0x4F, 0xE0, 0x3A, 0xEA, 0x10, 0x69, 0xA2, 0xD8, 0x08, 0x00, 0x2B, 0x30, 0x30,
        0x9D,
    ])?; // CLSID for My Computer

    // Drive item (e.g., C:)
    if let Some(drive) = target.components().next() {
        let drive_letter = drive.as_os_str().to_string_lossy();
        let drive_item = format!("{}:", drive_letter);
        let mut drive_buffer = Vec::new();
        drive_buffer.write_u16::<LE>((drive_item.len() + 1) as u16 * 2)?; // Size in bytes (including null terminator)
        drive_buffer.write_u8(0x31)?; // Type: PT_DRIVE (0x31)
        drive_buffer.write_u8(0x00)?; // Flags
        for c in drive_item.encode_utf16() {
            drive_buffer.write_u16::<LE>(c)?;
        }
        drive_buffer.write_u16::<LE>(0x0000)?; // Null terminator
        buffer.write_all(&drive_buffer)?;
    }

    // Path components
    for component in target.components().skip(1) {
        let component_str = component.as_os_str().to_string_lossy();
        let mut component_buffer = Vec::new();
        component_buffer.write_u16::<LE>((component_str.len() + 1) as u16 * 2)?; // Size in bytes (including null terminator)
        component_buffer.write_u8(0x31)?; // Type: PT_DRIVE (0x31) - This might need to be adjusted based on whether it's a file or directory
        component_buffer.write_u8(0x00)?; // Flags
        for c in component_str.encode_utf16() {
            component_buffer.write_u16::<LE>(c)?;
        }
        component_buffer.write_u16::<LE>(0x0000)?; // Null terminator
        buffer.write_all(&component_buffer)?;
    }

    // Terminal ID
    buffer.write_u16::<LE>(0x0000)?;

    writer.write_u16::<LE>(buffer.len() as u16)?;
    writer.write_all(&buffer)?;

    Ok(())
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
        const FILE_ATTRIBUTE_READONLY               = 0b1000_0000_0000_0000_0000_0000_0000_0000;

        /// The file or directory is hidden. If this bit is set, the file or folder is not included in an ordinary directory listing.
        const FILE_ATTRIBUTE_HIDDEN                 = 0b0100_0000_0000_0000_0000_0000_0000_0000;

        /// The file or directory is part of the operating system or is used exclusively by the operating system.
        const FILE_ATTRIBUTE_SYSTEM                 = 0b0010_0000_0000_0000_0000_0000_0000_0000;

        /// A bit that MUST be zero.
        const RESERVED_1                            = 0b0001_0000_0000_0000_0000_0000_0000_0000;

        /// The link target is a directory instead of a file.
        const FILE_ATTRIBUTE_DIRECTORY              = 0b0000_1000_0000_0000_0000_0000_0000_0000;

        /// The file or directory is an archive file. Applications use this flag to mark files for backup or removal.
        const FILE_ATTRIBUTE_ARCHIVE                = 0b0000_0100_0000_0000_0000_0000_0000_0000;

        /// A bit that MUST be zero.
        const RESERVED_2                            = 0b0000_0010_0000_0000_0000_0000_0000_0000;

        /// The file or directory has no other flags set. If this bit is 1, all other bits in this structure MUST be clear.
        const FILE_ATTRIBUTE_NORMAL                 = 0b0000_0001_0000_0000_0000_0000_0000_0000;

        /// The file is being used for temporary storage.
        const FILE_ATTRIBUTE_TEMPORARY              = 0b0000_0000_1000_0000_0000_0000_0000_0000;

        /// The file is a sparse file.
        const FILE_ATTRIBUTE_SPARCE_FILE            = 0b0000_0000_0100_0000_0000_0000_0000_0000;

        /// The file or directory has an associated reparse point.
        const FILE_ATTRIBUTE_REPARSE_POINT          = 0b0000_0000_0010_0000_0000_0000_0000_0000;

        /// The file or directory is compressed. For a file, this means that all data in the file is compressed. For a directory, this means that compression is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_COMPRESSED             = 0b0000_0000_0001_0000_0000_0000_0000_0000;

        /// The data of the file is not immediately available.
        const FILE_ATTRIBUTE_OFFLINE                = 0b0000_0000_0000_1000_0000_0000_0000_0000;

        /// The contents of the file need to be indexed.
        const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED    = 0b0000_0000_0000_0100_0000_0000_0000_0000;

        /// The file or directory is encrypted. For a file, this means that all data in the file is encrypted. For a directory, this means that encryption is the default for newly created files and subdirectories.
        const FILE_ATTRIBUTE_ENCRYPTED              = 0b0000_0000_0000_0010_0000_0000_0000_0000;

    }
}
