use std::path::PathBuf;

use crate::{
    config::{AppManifest, FilePayload, InstallConfig},
    installer::Installer,
    wizard::basic::BasicWizard,
};

pub type BasicWizardConfig = BasicWizardBuilder<PathBuf, AppManifest>;

impl BasicWizardConfig {
    pub fn build() -> BasicWizardBuilder<(), ()> {
        BasicWizardBuilder {
            install_path: (),
            manifest: (),
            payloads: Vec::new(),
        }
    }

    pub fn to_installer(self) -> Installer<BasicWizard> {
        let install_config = InstallConfig {
            install_path: self.install_path,
            payloads: self.payloads,
            create_start_menu_shortcut: true,
            create_desktop_shortcut: false,
        };
        let wizard = BasicWizard::from_config(install_config, self.manifest);
        Installer::from_wizard(wizard)
    }
}

pub struct BasicWizardBuilder<A, B> {
    pub(super) install_path: A,
    manifest: B,
    payloads: Vec<FilePayload>,
}

impl<A, B> BasicWizardBuilder<A, B> {
    pub fn default_install_path(self, path: impl Into<PathBuf>) -> BasicWizardBuilder<PathBuf, B> {
        BasicWizardBuilder {
            install_path: path.into(),
            manifest: self.manifest,
            payloads: self.payloads,
        }
    }

    pub fn manifest(self, manifest: AppManifest) -> BasicWizardBuilder<A, AppManifest> {
        BasicWizardBuilder {
            install_path: self.install_path,
            manifest,
            payloads: self.payloads,
        }
    }

    pub fn add_payload(mut self, payload: FilePayload) -> BasicWizardBuilder<A, B> {
        self.payloads.push(payload);
        self
    }
}
