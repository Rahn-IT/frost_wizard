use std::io::{Read, Write};

use crate::lnk::{
    LnkWriteError,
    block_data::{
        console::{Console, ConsoleDataBlockParseError},
        icon_environment::{IconEnvironment, IconEnvironmentDataBlockParseError},
        known_folder::{KnownFolder, KnownFolderDataBlockParseError},
        property_store::PropertyStore,
        special_folder::{SpecialFolder, SpecialFolderDataBlockParseError},
        tracker::{Tracker, TrackerDataBlockParseError},
    },
    helpers::read_u32,
};

mod console;
mod icon_environment;
mod known_folder;
mod property_store;
mod special_folder;
mod tracker;

#[derive(Debug, thiserror::Error)]
pub enum BlockDataParseError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("unknown data block signature: {0:08x}")]
    UnknownDataBlockSignature(u32),
    #[error("unparsed block data: {0:?}")]
    UnparsedBlockData(Vec<u8>),
    #[error("error while parsing console data block: {0}")]
    ConsoleDataBlockError(#[from] ConsoleDataBlockParseError),
    #[error("error while parsing tracker data block: {0}")]
    TrackerDataBlockError(#[from] TrackerDataBlockParseError),
    #[error("error while parsing property store data block: {0}")]
    PropertyStoreDataBlockError(#[from] property_store::PropertyStoreDataBlockParseError),
    #[error("error while parsing icon environment data block: {0}")]
    IconEnvironmentDataBlockerror(#[from] IconEnvironmentDataBlockParseError),
    #[error("error while parsing special folder data block: {0}")]
    SpecialFolderDataBlockError(#[from] SpecialFolderDataBlockParseError),
    #[error("error while parsing known folder data block: {0}")]
    KnownFolderDataBlockError(#[from] KnownFolderDataBlockParseError),
}

#[derive(Debug)]
pub struct BlockData {
    pub console: Option<Console>,
    pub tracker: Option<Tracker>,
    pub icon_environment: Option<IconEnvironment>,
    pub special_folders: Vec<SpecialFolder>,
    pub known_folders: Vec<KnownFolder>,
    pub property_store: PropertyStore,
}

impl BlockData {
    pub fn parse(data: &mut impl Read) -> Result<Self, BlockDataParseError> {
        let mut me = Self {
            console: None,
            tracker: None,
            icon_environment: None,
            special_folders: Vec::new(),
            known_folders: Vec::new(),
            property_store: PropertyStore::default(),
        };

        loop {
            let block_size = read_u32(data)?;
            println!("Block size: {block_size}");
            if block_size <= 0x4 {
                // Termination Block
                break;
            }
            let signature = read_u32(data)?;
            let signature = BlockSignature::from_u32(signature)
                .ok_or_else(|| BlockDataParseError::UnknownDataBlockSignature(signature))?;
            let mut block_data = data.take(block_size as u64 - 8);
            println!("signature: {signature:?}");

            match signature {
                BlockSignature::ConsoleDataBlock => {
                    let console_data_block = Console::parse(&mut block_data)?;
                    me.console = Some(console_data_block);
                }
                BlockSignature::TrackerDataBlock => {
                    let tracker_data_block = Tracker::parse(&mut block_data)?;
                    me.tracker = Some(tracker_data_block);
                }
                BlockSignature::PropertyStoreDataBlock => {
                    me.property_store.parse(&mut block_data)?
                }
                BlockSignature::IconEnvironmentDataBlock => {
                    let icon_environment = IconEnvironment::parse(&mut block_data)?;
                    me.icon_environment = Some(icon_environment);
                }
                BlockSignature::SpecialFolderDataBlock => {
                    let special_folder = SpecialFolder::parse(&mut block_data)?;
                    me.special_folders.push(special_folder);
                }
                BlockSignature::KnownFolderDataBlock => {
                    let known_folder = KnownFolder::parse(&mut block_data)?;
                    me.known_folders.push(known_folder);
                }
                _ => todo!(),
            };

            let mut remaining_data = Vec::new();
            if block_data.read_to_end(&mut remaining_data)? > 0 {
                return Err(BlockDataParseError::UnparsedBlockData(remaining_data));
            }
        }
        Ok(me)
    }

    pub(crate) fn write(&self, data: &mut impl Write) -> Result<(), LnkWriteError> {
        todo!()
    }
}

#[derive(Debug)]
pub enum BlockSignature {
    ConsoleDataBlock,
    ConsoleFEDataBlock,
    DarwinDataBlock,
    EnvironmentVariableDataBlock,
    IconEnvironmentDataBlock,
    KnownFolderDataBlock,
    PropertyStoreDataBlock,
    ShimDataBlock,
    SpecialFolderDataBlock,
    TrackerDataBlock,
    VistaAndAboveIDListDataBlock,
}

impl BlockSignature {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0xA0000002 => Some(BlockSignature::ConsoleDataBlock),
            0xA0000004 => Some(BlockSignature::ConsoleFEDataBlock),
            0xA0000006 => Some(BlockSignature::DarwinDataBlock),
            0xA0000001 => Some(BlockSignature::EnvironmentVariableDataBlock),
            0xA0000007 => Some(BlockSignature::IconEnvironmentDataBlock),
            0xA000000B => Some(BlockSignature::KnownFolderDataBlock),
            0xA0000009 => Some(BlockSignature::PropertyStoreDataBlock),
            0xA0000008 => Some(BlockSignature::ShimDataBlock),
            0xA0000005 => Some(BlockSignature::SpecialFolderDataBlock),
            0xA0000003 => Some(BlockSignature::TrackerDataBlock),
            0xA000000C => Some(BlockSignature::VistaAndAboveIDListDataBlock),
            _ => None,
        }
    }
}
