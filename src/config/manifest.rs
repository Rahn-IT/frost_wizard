// pub struct ApplicationManifest {
//     pub name: String,
//     pub version: String,
//     pub icon: Option<Vec<u8>>,
// }

use serde::{Deserialize, Serialize};

pub type AppManifest = AppManifestBuilder<String, String>;

impl AppManifest {
    pub fn build() -> AppManifestBuilder<(), ()> {
        AppManifestBuilder {
            name: (),
            version: (),
            publisher: None,
            icon: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppManifestBuilder<A, B> {
    pub(crate) name: A,
    pub(crate) version: B,
    pub(crate) publisher: Option<String>,
    pub(crate) icon: Option<Vec<u8>>,
}

impl<A, B> AppManifestBuilder<A, B> {
    pub fn name(self, name: impl Into<String>) -> AppManifestBuilder<String, B> {
        AppManifestBuilder {
            name: name.into(),
            version: self.version,
            publisher: self.publisher,
            icon: self.icon,
        }
    }

    pub fn version(self, version: impl Into<String>) -> AppManifestBuilder<A, String> {
        AppManifestBuilder {
            name: self.name,
            version: version.into(),
            publisher: self.publisher,
            icon: self.icon,
        }
    }

    pub fn publisher(self, publisher: impl Into<String>) -> AppManifestBuilder<A, B> {
        AppManifestBuilder {
            name: self.name,
            version: self.version,
            publisher: Some(publisher.into()),
            icon: self.icon,
        }
    }

    pub fn icon(self, icon: Vec<u8>) -> AppManifestBuilder<A, B> {
        AppManifestBuilder {
            name: self.name,
            version: self.version,
            publisher: self.publisher,
            icon: Some(icon),
        }
    }
}
