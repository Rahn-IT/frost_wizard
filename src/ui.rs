use iced::{
    Element, Task,
    widget::{horizontal_space, row, text},
};

use crate::{AppManifest, ui::scaffold::Scaffold};

pub mod scaffold;

#[derive(PartialEq)]
pub enum InstallerStep {
    Introduction,
    Wizard,
    Installing,
    Completed,
}

#[derive(Debug, Clone)]
pub enum Message<WizardMessage> {
    Next,
    Back,
    Wizard(WizardMessage),
}

pub struct InstallerUi<Wizard> {
    step: InstallerStep,
    wizard: Wizard,
    manifest: AppManifest,
}

impl<Wizard> InstallerUi<Wizard>
where
    Wizard: crate::wizard::Wizard,
{
    pub fn start(wizard: Wizard, manifest: AppManifest) -> (Self, Task<Message<Wizard::Message>>) {
        let ui = Self {
            step: InstallerStep::Introduction,
            wizard,
            manifest,
        };
        (ui, Task::none())
    }

    pub fn update(&mut self, message: Message<Wizard::Message>) -> Task<Message<Wizard::Message>> {
        match message {
            Message::Wizard(message) => {
                if self.step == InstallerStep::Wizard {
                    match self.wizard.update(message) {
                        crate::wizard::WizardAction::None => Task::none(),
                        crate::wizard::WizardAction::Run(task) => task.map(Message::Wizard),
                        crate::wizard::WizardAction::Back => {
                            self.step = InstallerStep::Introduction;
                            Task::none()
                        }
                        crate::wizard::WizardAction::Install(config) => todo!(),
                    }
                } else {
                    Task::none()
                }
            }
            Message::Next => match &mut self.step {
                InstallerStep::Introduction => {
                    self.step = InstallerStep::Wizard;
                    Task::none()
                }
                InstallerStep::Wizard => Task::none(),
                InstallerStep::Installing => {
                    todo!();
                    Task::none()
                }
                InstallerStep::Completed => {
                    todo!();
                    Task::none()
                }
            },
            Message::Back => {
                todo!();
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message<Wizard::Message>> {
        match &self.step {
            InstallerStep::Introduction => Scaffold::new()
                .title(row![text(&self.manifest.name).size(24), horizontal_space(), text(&self.manifest.version).size(24)])
                .control(text(format!(
                    "Welcome to the installation wizard for {}!",
                    self.manifest.name
                )))
                .control(text("This wizard will guide your through the installation process and help you keep a cool head."))
                .on_next(Message::Next)
                .into(),
            InstallerStep::Wizard => self.wizard.view().map(Message::Wizard),
            InstallerStep::Installing => todo!(),
            InstallerStep::Completed => todo!(),
        }
    }

    pub fn title(&self) -> String {
        // TODO: configurable title
        "Iced Installer".to_string()
    }
}
