use std::{io::Read, sync::Arc};

use iced::{
    Element, Task, exit,
    widget::{horizontal_space, progress_bar, row, text},
};
use sipper::{Sipper, StreamExt, sipper};
use thiserror::Error;
use tokio::{fs, io::AsyncWriteExt};
use zip::{ZipArchive, result::ZipError};

use crate::{AppManifest, FilePayload, config::InstallConfig, ui::scaffold::Scaffold};

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

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("Failed to create install directory:\n{0}")]
    CreateInstallDir(std::io::Error),
    #[error("Failed to write payload into install directory:\n{0}")]
    WritePayload(std::io::Error),
    #[error("Failed to extract payload into install directory:\n{0}")]
    ZipError(ZipError),
    #[error("Unknown zip size")]
    UnknownZipSize,
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
                    match self.wizard.update(message) {
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
        let sipper = sipper(|mut sender| async move {
            let config = config;

            fs::create_dir_all(&config.install_path)
                .await
                .map_err(InstallError::CreateInstallDir)?;

            let mut full_size = 0u64;

            for payload in &config.payloads {
                match payload.as_ref() {
                    FilePayload::File { contents, .. } => full_size += contents.len() as u64,
                    FilePayload::Directory { data } => {
                        full_size += ZipArchive::new(std::io::Cursor::new(data))
                            .map_err(InstallError::ZipError)?
                            .decompressed_size()
                            .ok_or(InstallError::UnknownZipSize)?
                            as u64;
                    }
                }
            }

            let full_size = full_size as f32;

            let mut written = 0u64;

            for payload in &config.payloads {
                match payload.as_ref() {
                    FilePayload::File { name, contents } => {
                        let path = config.install_path.join(name.as_ref());
                        fs::write(path, contents)
                            .await
                            .map_err(InstallError::WritePayload)?;

                        written += contents.len() as u64;

                        sender
                            .send(Message::Progress(written as f32 / full_size))
                            .await;
                    }
                    FilePayload::Directory { data } => {
                        let mut zip = ZipArchive::new(std::io::Cursor::new(data))
                            .map_err(InstallError::ZipError)?;

                        for index in 0..zip.len() {
                            let mut reader = zip.by_index(index).map_err(InstallError::ZipError)?;
                            let path = config.install_path.join(reader.name());
                            if let Some(parent) = path.parent() {
                                fs::create_dir_all(parent)
                                    .await
                                    .map_err(InstallError::WritePayload)?;
                            }
                            let mut file = fs::File::create(path)
                                .await
                                .map_err(InstallError::WritePayload)?;

                            let mut buf = [0; 1024];

                            loop {
                                let n =
                                    reader.read(&mut buf).map_err(InstallError::WritePayload)?;
                                if n == 0 {
                                    break;
                                }

                                file.write_all(&buf[..n])
                                    .await
                                    .map_err(InstallError::WritePayload)?;

                                written += n as u64;

                                sender
                                    .send(Message::Progress(written as f32 / full_size))
                                    .await;
                            }
                        }
                    }
                }
            }

            sender.send(Message::Progress(1.0)).await;

            Ok(Message::InstallDone)
        })
        .with(|message| Ok(message));

        Task::stream(sipper::stream(sipper).map(|result| match result {
            Ok(message) => message,
            Err(error) => Message::InstallError(error),
        }))
    }
}
