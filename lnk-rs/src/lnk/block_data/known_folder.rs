use std::io::Read;

use crate::lnk::helpers::{Guid, read_guid, read_u32};

#[derive(Debug, thiserror::Error)]
pub enum KnownFolderDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unknown known-folder GUID: {0:?}")]
    UnknownKnownFolder(Guid),
}

#[derive(Debug, Clone)]
pub struct KnownFolder {
    /// KNOWNFOLDERID (GUID) identifying the folder.
    pub folder: KnownFolderType,
    /// Offset into the LinkTargetIDList that, when combined with the folder, locates the item.
    pub offset: u32,
}

impl KnownFolder {
    /// `data` must point right after BlockSize + BlockSignature.
    /// Reads exactly: KnownFolderID (16 bytes) + Offset (u32 LE).
    pub fn parse(data: &mut impl Read) -> Result<Self, KnownFolderDataBlockParseError> {
        let guid = read_guid(data)?;
        let offset = read_u32(data)?;

        let folder = KnownFolderType::from_guid(&guid)
            .ok_or_else(|| KnownFolderDataBlockParseError::UnknownKnownFolder(guid))?;

        Ok(Self { folder, offset })
    }
}

/// Well-known folder identifiers (KNOWNFOLDERIDs from Windows)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownFolderType {
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
    AppData,
    LocalAppData,
    ProgramFiles,
    ProgramFilesX86,
    Windows,
    PublicDesktop,
    CommonStartMenu,
    CommonPrograms,
    StartMenu,
    Startup,
    QuickLaunch,
    OneDrive,
    Profile,
}

impl KnownFolderType {
    /// Try to map a GUID to a well-known folder constant.
    pub fn from_guid(guid: &Guid) -> Option<Self> {
        match guid.to_string().to_uppercase().as_str() {
            "B4BFCC3A-DB2C-424C-B029-7FE99A87C641" => Some(Self::Desktop),
            "FDD39AD0-238F-46AF-ADB4-6C85480369C7" => Some(Self::Documents),
            "374DE290-123F-4565-9164-39C4925E467B" => Some(Self::Downloads),
            "33E28130-4E1E-4676-835A-98395C3BC3BB" => Some(Self::Pictures),
            "4BD8D571-6D19-48D3-BE97-422220080E43" => Some(Self::Music),
            "18989B1D-99B5-455B-841C-AB7C74E4DDFC" => Some(Self::Videos),
            "3EB685DB-65F9-4CF6-A03A-E3EF65729F3D" => Some(Self::AppData),
            "F1B32785-6FBA-4FCF-9D55-7B8E7F157091" => Some(Self::LocalAppData),
            "905E63B6-C1BF-494E-B29C-65B732D3D21A" => Some(Self::ProgramFiles),
            "7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E" => Some(Self::ProgramFilesX86),
            "F38BF404-1D43-42F2-9305-67DE0B28FC23" => Some(Self::Windows),
            "C4AA340D-F20F-4863-AFEF-F87EF2E6BA25" => Some(Self::PublicDesktop),
            "A4115719-D62E-491D-AA7C-E74B8BE3B067" => Some(Self::CommonStartMenu),
            "0139D44E-6AFE-49F2-8690-3DAFCAE6FFB8" => Some(Self::CommonPrograms),
            "625B53C3-AB48-4EC1-BA1F-A1EF4146FC19" => Some(Self::StartMenu),
            "B97D20BB-F46A-4C97-BA10-5E3608430854" => Some(Self::Startup),
            "52A4F021-7B75-48A9-9F6B-4B87A210BC8F" => Some(Self::QuickLaunch),
            "A52BBA46-E9E1-435F-B3D9-28DAA648C0F6" => Some(Self::OneDrive),
            "5E6C858F-0E22-4760-9AFE-EA3317B67173" => Some(Self::Profile),
            _ => None,
        }
    }
}
