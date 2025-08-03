use std::path::PathBuf;

use iced::{
    Alignment::Center,
    Task,
    widget::{button, row, text},
};
use rfd::AsyncFileDialog;

use crate::{
    config::InstallConfig,
    installer::basic::config::{BasicWizardBuilder, BasicWizardConfig},
    ui::scaffold::Scaffold,
    wizard::{Wizard, WizardAction},
};

mod config;

pub struct BasicWizard {
    config: InstallConfig,
    install_path_display: String,
}

impl BasicWizard {
    pub fn builder() -> BasicWizardBuilder<(), ()> {
        BasicWizardConfig::build()
    }

    fn from_config(config: InstallConfig) -> Self {
        BasicWizard {
            install_path_display: config.install_path.display().to_string(),
            config,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectInstallPath,
    SetInstallPath(Option<PathBuf>),
    Back,
    Next,
}

impl Wizard for BasicWizard {
    type Message = Message;

    fn start(&self) -> WizardAction<Self::Message> {
        WizardAction::None
    }

    fn update(&mut self, message: Self::Message) -> crate::wizard::WizardAction<Self::Message> {
        match message {
            Message::SelectInstallPath => {
                self.install_path_display = "Selecting...".to_string();
                let task = Task::perform(
                    async {
                        AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::SetInstallPath,
                );
                WizardAction::Run(task)
            }
            Message::SetInstallPath(path) => {
                if let Some(path) = path {
                    self.config.install_path = path;
                }
                self.install_path_display = self.config.install_path.display().to_string();
                WizardAction::None
            }

            Message::Back => WizardAction::Back,
            Message::Next => WizardAction::Install(self.config.clone()),
        }
    }

    fn view(&self) -> iced::Element<Self::Message> {
        Scaffold::new()
            .title("Select install location")
            .control(
                row![
                    button("Select Folder").on_press(Message::SelectInstallPath),
                    text(&self.install_path_display)
                ]
                .spacing(20)
                .align_y(Center),
            )
            .on_next(Message::Next)
            .on_back(Message::Back)
            .into()
    }
}
