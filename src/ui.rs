use std::sync::Arc;

use iced::{
    Element, Task, exit,
    widget::{horizontal_space, progress_bar, row, text},
};
use sipper::Sipper;

use crate::{
    AppManifest, config::InstallConfig, installer::InstallError, ui::scaffold::Scaffold,
    wizard::WizardAction,
};

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
    Wizard(WizardMessage),
    Progress(f32),
    InstallDone,
    InstallError(Arc<InstallError>),
    Finish,
}

impl<WizardMessage> From<InstallError> for Message<WizardMessage> {
    fn from(error: InstallError) -> Self {
        Message::InstallError(Arc::new(error))
    }
}

pub struct InstallerUi<Wizard> {
    step: InstallerStep,
    wizard: Wizard,
    manifest: AppManifest,
    progress: f32,
    finished: bool,
    error: Option<Arc<InstallError>>,
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
            progress: 0.0,
            finished: false,
            error: None,
        };
        (ui, Task::none())
    }

    pub fn update(&mut self, message: Message<Wizard::Message>) -> Task<Message<Wizard::Message>> {
        match message {
            Message::Wizard(message) => {
                if self.step == InstallerStep::Wizard {
                    let action = self.wizard.update(message);
                    self.handle_action(action)
                } else {
                    Task::none()
                }
            }
            Message::Next => match &mut self.step {
                InstallerStep::Introduction => {
                    self.step = InstallerStep::Wizard;
                    let action = self.wizard.start();
                    self.handle_action(action)
                }
                InstallerStep::Wizard => Task::none(),
                InstallerStep::Installing => {
                    self.step = InstallerStep::Completed;
                    Task::none()
                }
                InstallerStep::Completed => Task::none(),
            },
            Message::Progress(progress) => {
                self.progress = progress;
                Task::none()
            }
            Message::InstallDone => {
                self.finished = true;
                self.progress = 1.0;
                Task::none()
            }
            Message::InstallError(error) => {
                self.error = Some(error);
                Task::none()
            }
            Message::Finish => exit(),
        }
    }

    fn handle_action(
        &mut self,
        action: WizardAction<Wizard::Message>,
    ) -> Task<Message<Wizard::Message>> {
        match action {
            crate::wizard::WizardAction::None => Task::none(),
            crate::wizard::WizardAction::Run(task) => task.map(Message::Wizard),
            crate::wizard::WizardAction::Back => {
                self.step = InstallerStep::Introduction;
                Task::none()
            }
            crate::wizard::WizardAction::Install(config) => {
                self.step = InstallerStep::Installing;
                self.progress = 0.0;
                self.install(config)
            }
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message<Wizard::Message>> {
        if let Some(_error) = &self.error {
            todo!()
        }

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
            InstallerStep::Wizard => {
                self.wizard.view().map(Message::Wizard)
            },
            InstallerStep::Installing => {
                Scaffold::new()
                                .title(row![text(&self.manifest.name).size(24), horizontal_space(), text(&self.manifest.version).size(24)])
                                .control(text(format!(
                                    "Installing {}!",
                                    self.manifest.name
                                )))
                                .control(progress_bar(0.0..=1.0, self.progress))
                                .control(text(format!("{:.0}%", self.progress * 100.0)))
                                .on_next_maybe(self.finished.then_some(Message::Next))
                                .into()
            },
            InstallerStep::Completed =>
            Scaffold::new()
                            .title(row![text(&self.manifest.name).size(24), horizontal_space(), text(&self.manifest.version).size(24)])
                            .control(text(format!(
                                "{} installed successfully!",
                                self.manifest.name
                            )))
                            .on_finish(Message::Finish)
                            .into()
,
        }
    }

    pub fn title(&self) -> String {
        // TODO: configurable title
        "Iced Installer".to_string()
    }

    fn install(&self, config: InstallConfig) -> Task<Message<Wizard::Message>> {
        let sipper =
            crate::installer::install(config, self.manifest.clone(), |result| match result {
                Ok(()) => Message::InstallDone,
                Err(error) => Message::InstallError(Arc::new(error)),
            })
            .with(|message| Message::Progress(message));

        Task::stream(sipper::stream(sipper))
    }
}
