use std::fmt::Debug;

use iced::Task;

use crate::config::InstallConfig;

pub enum WizardAction<Message> {
    None,
    Run(Task<Message>),
    Back,
    Install(InstallConfig),
}

pub trait Wizard: Sized + 'static {
    type Message: Clone + Debug + Send;
    fn start(&self) -> Task<Self::Message>;
    fn update(&mut self, message: Self::Message) -> WizardAction<Self::Message>;
    fn view(&self) -> iced::Element<Self::Message>;
}
