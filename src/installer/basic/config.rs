use std::{path::PathBuf, sync::Arc};

use crate::{
    BasicWizard,
    config::{AppManifest, FilePayload, InstallConfig},
    installer::Installer,
};

pub type BasicWizardConfig = BasicWizardBuilder<PathBuf, AppManifest, Arc<FilePayload>>;

impl BasicWizardConfig {
    pub fn build() -> BasicWizardBuilder<(), (), ()> {
        BasicWizardBuilder {
            install_path: (),
            manifest: (),
            payload: (),
        }
    }

    pub fn to_installer(self) -> Installer<BasicWizard> {
        let install_config = InstallConfig {
            install_path: self.install_path,
            payloads: vec![self.payload],
        };
        let wizard = BasicWizard::from_config(install_config);
        Installer::from_wizard(wizard, self.manifest)
    }
}

#[derive(Clone)]
pub struct BasicWizardBuilder<A, B, C> {
    pub(super) install_path: A,
    manifest: B,
    payload: C,
}

impl<A, B, C> BasicWizardBuilder<A, B, C> {
    pub fn default_install_path(
        self,
        path: impl Into<PathBuf>,
    ) -> BasicWizardBuilder<PathBuf, B, C> {
        BasicWizardBuilder {
            install_path: path.into(),
            manifest: self.manifest,
            payload: self.payload,
        }
    }

    pub fn manifest(self, manifest: AppManifest) -> BasicWizardBuilder<A, AppManifest, C> {
        BasicWizardBuilder {
            install_path: self.install_path,
            manifest,
            payload: self.payload,
        }
    }

    pub fn payload(self, payload: FilePayload) -> BasicWizardBuilder<A, B, Arc<FilePayload>> {
        BasicWizardBuilder {
            install_path: self.install_path,
            manifest: self.manifest,
            payload: Arc::new(payload),
        }
    }
}
