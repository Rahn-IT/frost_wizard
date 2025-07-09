use std::path::PathBuf;
use std::{borrow::Cow, sync::Arc};

mod manifest;
pub use manifest::AppManifest;

#[derive(Clone)]
pub struct InstallConfig {
    pub install_path: PathBuf,
    pub payload: Arc<FilePayload>,
}

pub enum FilePayload {
    /// The contents of a file
    File {
        name: Cow<'static, str>,
        contents: Cow<'static, [u8]>,
    },
    /// A tar packed directory
    Directory(Cow<'static, str>),
}
