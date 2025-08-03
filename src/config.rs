use std::path::PathBuf;
use std::{borrow::Cow, sync::Arc};

mod manifest;
pub use manifest::AppManifest;

#[derive(Clone)]
pub struct InstallConfig {
    pub install_path: PathBuf,
    pub payloads: Vec<Arc<FilePayload>>,
}

pub enum FilePayload {
    /// The contents of a file
    File {
        name: Cow<'static, str>,
        contents: Cow<'static, [u8]>,
    },
    /// A zip packed directory
    Directory { data: Cow<'static, [u8]> },
}

#[macro_export]
macro_rules! embed_directory {
    ($path:expr) => {
        FilePayload::Directory {
            data: std::borrow::Cow::Borrowed(macros::include_dir_zip!($path)),
        }
    };
}
