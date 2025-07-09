use crate::{AppManifest, ui::InstallerUi};

pub mod basic;

pub struct Installer<Wizard> {
    manifest: AppManifest,
    wizard: Wizard,
}

impl<Wizard> Installer<Wizard>
where
    Wizard: crate::wizard::Wizard,
{
    pub fn from_wizard(wizard: Wizard, manifest: AppManifest) -> Self {
        Self { wizard, manifest }
    }

    pub fn run(self) -> Result<(), iced::Error> {
        iced::application(
            InstallerUi::<Wizard>::title,
            InstallerUi::<Wizard>::update,
            InstallerUi::<Wizard>::view,
        )
        .run_with(move || InstallerUi::start(self.wizard, self.manifest))
    }
}
