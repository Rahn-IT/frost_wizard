use std::borrow::Cow;
use std::path::PathBuf;

mod manifest;
pub use manifest::AppManifest;

pub struct InstallConfig {
    pub install_path: PathBuf,
    pub payloads: Vec<FilePayload>,
}

pub enum FilePayload {
    /// The contents of a file
    File {
        name: Cow<'static, str>,
        contents: Cow<'static, [u8]>,
    },
    /// A zip packed directory
    Directory {
        unpacked_size: u64,
        reader: Box<dyn DirTrait + Send + Sync>,
    },
}

pub trait DirTrait: std::io::Read + std::io::Seek {}

impl<T> DirTrait for T where T: std::io::Read + std::io::Seek {}

#[macro_export]
macro_rules! embed_directory {
    ($path:expr) => {{
        let data = macros::include_dir_zip!($path);

        frost_wizard::config::FilePayload::Directory {
            reader: Box::new(std::io::Cursor::new(data)),
            unpacked_size: data.len() as u64,
        }
    }};
}
