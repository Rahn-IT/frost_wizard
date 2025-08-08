// pub struct ApplicationManifest {
//     pub name: String,
//     pub version: String,
//     pub icon: Option<Vec<u8>>,
// }

use serde::{Deserialize, Serialize};

pub type AppManifest = AppManifestBuilder<String, String, String>;

impl AppManifest {
    pub fn build() -> AppManifestBuilder<(), (), ()> {
        AppManifestBuilder {
            friendly_name: (),
            version: (),
            bin_name: (),
            publisher: None,
            icon: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppManifestBuilder<A, B, C> {
    pub friendly_name: A,
    pub version: B,
    pub bin_name: C,
    pub publisher: Option<String>,
    pub icon: Option<Vec<u8>>,
}

impl<A, B, C> AppManifestBuilder<A, B, C> {
    pub fn friendly_name(self, name: impl Into<String>) -> AppManifestBuilder<String, B, C> {
        AppManifestBuilder {
            friendly_name: name.into(),
            version: self.version,
            bin_name: self.bin_name,
            publisher: self.publisher,
            icon: self.icon,
        }
    }

    pub fn version(self, version: impl Into<String>) -> AppManifestBuilder<A, String, C> {
        AppManifestBuilder {
            friendly_name: self.friendly_name,
            version: version.into(),
            bin_name: self.bin_name,
            publisher: self.publisher,
            icon: self.icon,
        }
    }

    pub fn bin_name(self, bin_name: impl Into<String>) -> AppManifestBuilder<A, B, String> {
        AppManifestBuilder {
            friendly_name: self.friendly_name,
            version: self.version,
            bin_name: bin_name.into(),
            publisher: self.publisher,
            icon: self.icon,
        }
    }

    pub fn publisher(self, publisher: impl Into<String>) -> AppManifestBuilder<A, B, C> {
        AppManifestBuilder {
            friendly_name: self.friendly_name,
            version: self.version,
            bin_name: self.bin_name,
            publisher: Some(publisher.into()),
            icon: self.icon,
        }
    }

    pub fn icon(self, icon: Vec<u8>) -> AppManifestBuilder<A, B, C> {
        AppManifestBuilder {
            friendly_name: self.friendly_name,
            version: self.version,
            bin_name: self.bin_name,
            publisher: self.publisher,
            icon: Some(icon),
        }
    }
}
