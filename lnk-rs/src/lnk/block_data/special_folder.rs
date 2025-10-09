use std::io::{self, Read};
use thiserror::Error;

use crate::lnk::helpers::read_u32;

#[derive(Debug, Error)]
pub enum SpecialFolderDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("unknown special-folder ID: {0}")]
    UnknownSpecialFolder(u32),
}

#[derive(Debug, Clone)]
pub struct SpecialFolder {
    /// Known CSIDL special folder identifier.
    pub folder: SpecialFolderType,
    /// Offset into the LinkTargetIDList that, when combined with the folder, locates the item.
    pub offset: u32,
}

impl SpecialFolder {
    /// `data` must point right after BlockSize + BlockSignature.
    /// Reads exactly 8 bytes: SpecialFolderID (u32 LE), Offset (u32 LE).
    pub fn parse(data: &mut impl Read) -> Result<Self, SpecialFolderDataBlockParseError> {
        let id = read_u32(data)?;
        let offset = read_u32(data)?;

        let folder = SpecialFolderType::from_id(id)
            .ok_or(SpecialFolderDataBlockParseError::UnknownSpecialFolder(id))?;

        Ok(Self { folder, offset })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialFolderType {
    Desktop,
    Internet,
    Programs,
    Controls,
    Printers,
    Personal,
    Favorites,
    Startup,
    Recent,
    SendTo,
    BitBucket,
    StartMenu,
    MyMusic,
    MyVideo,
    DesktopDirectory,
    MyComputer,
    NetworkNeighborhood,
    NetHood,
    Fonts,
    Templates,
    CommonStartMenu,
    CommonPrograms,
    CommonStartup,
    CommonDesktopDirectory,
    AppData,
    PrintHood,
    LocalAppData,
    AltStartup,
    CommonAltStartup,
    CommonFavorites,
    InternetCache,
    Cookies,
    History,
    CommonAppData,
    Windows,
    System,
    ProgramFiles,
    MyPictures,
    Profile,
    SystemX86,
    ProgramFilesX86,
    ProgramFilesCommon,
    ProgramFilesCommonX86,
    CommonTemplates,
    CommonDocuments,
    CommonAdminTools,
    AdminTools,
    Connections,
    CommonMusic,
    CommonPictures,
    CommonVideo,
    Resources,
    ResourcesLocalized,
    CommonOemLinks,
    CDBurnArea,
    ComputersNearMe,
}

impl SpecialFolderType {
    pub fn from_id(id: u32) -> Option<Self> {
        Some(match id {
            0x0000 => Self::Desktop,
            0x0001 => Self::Internet,
            0x0002 => Self::Programs,
            0x0003 => Self::Controls,
            0x0004 => Self::Printers,
            0x0005 => Self::Personal, // "My Documents"
            0x0006 => Self::Favorites,
            0x0007 => Self::Startup,
            0x0008 => Self::Recent,
            0x0009 => Self::SendTo,
            0x000A => Self::BitBucket, // Recycle Bin
            0x000B => Self::StartMenu,
            0x000D => Self::MyMusic,
            0x000E => Self::MyVideo,
            0x0010 => Self::DesktopDirectory,
            0x0011 => Self::MyComputer,
            0x0012 => Self::NetworkNeighborhood,
            0x0013 => Self::NetHood,
            0x0014 => Self::Fonts,
            0x0015 => Self::Templates,
            0x0016 => Self::CommonStartMenu,
            0x0017 => Self::CommonPrograms,
            0x0018 => Self::CommonStartup,
            0x0019 => Self::CommonDesktopDirectory,
            0x001A => Self::AppData,
            0x001B => Self::PrintHood,
            0x001C => Self::LocalAppData,
            0x001D => Self::AltStartup,
            0x001E => Self::CommonAltStartup,
            0x001F => Self::CommonFavorites,
            0x0020 => Self::InternetCache,
            0x0021 => Self::Cookies,
            0x0022 => Self::History,
            0x0023 => Self::CommonAppData,
            0x0024 => Self::Windows,
            0x0025 => Self::System,
            0x0026 => Self::ProgramFiles,
            0x0027 => Self::MyPictures,
            0x0028 => Self::Profile,
            0x0029 => Self::SystemX86,
            0x002A => Self::ProgramFilesX86, // ← your ID 42
            0x002B => Self::ProgramFilesCommon,
            0x002C => Self::ProgramFilesCommonX86,
            0x002D => Self::CommonTemplates,
            0x002E => Self::CommonDocuments,
            0x002F => Self::CommonAdminTools,
            0x0030 => Self::AdminTools,
            0x0031 => Self::Connections,
            0x0035 => Self::CommonMusic,
            0x0036 => Self::CommonPictures,
            0x0037 => Self::CommonVideo,
            0x0038 => Self::Resources,
            0x0039 => Self::ResourcesLocalized,
            0x003A => Self::CommonOemLinks,
            0x003B => Self::CDBurnArea,
            0x003D => Self::ComputersNearMe,
            _ => return None,
        })
    }
}
