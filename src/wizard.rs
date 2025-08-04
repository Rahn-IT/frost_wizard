use std::fmt::Debug;

use iced::Task;

use crate::{AppManifest, config::InstallConfig};

pub enum WizardAction<Message> {
    None,
    Run(Task<Message>),
    Back,
    Install(InstallConfig),
}

pub trait Wizard: Sized + 'static {
    type Message: Clone + Debug + Send;

    /// Called before starting the graphical wizard
    /// Here you can load the app manifest
    fn get_manifest(&self) -> AppManifest;
    /// Called before starting the graphical wizard
    /// You can return Some(InstallConfig) to skip the wizard and install the system directly
    /// This is useful to allow unattended installation with e.g. a silent flag
    fn unattended_install(&self) -> Option<InstallConfig>;
    /// Called when the wizard is first shown
    fn start(&self) -> WizardAction<Self::Message>;
    /// Iced update method for the wizard
    fn update(&mut self, message: Self::Message) -> WizardAction<Self::Message>;
    /// Iced view method for the wizard
    fn view(&self) -> iced::Element<Self::Message>;
}
