use std::path::PathBuf;

use clap::Parser;
use iced::{
    Alignment::Center,
    Task,
    widget::{button, horizontal_space, row, text},
};
use rfd::AsyncFileDialog;

use crate::{
    AppManifest,
    config::InstallConfig,
    ui::scaffold::Scaffold,
    wizard::{
        Wizard, WizardAction,
        basic::config::{BasicWizardBuilder, BasicWizardConfig},
    },
};

mod config;

pub struct BasicWizard {
    config: Option<InstallConfig>,
    install_path_display: String,
    manifest: AppManifest,
}

impl BasicWizard {
    pub fn builder() -> BasicWizardBuilder<(), ()> {
        BasicWizardConfig::build()
    }

    fn from_config(config: InstallConfig, manifest: AppManifest) -> Self {
        BasicWizard {
            install_path_display: config.install_path.display().to_string(),
            config: Some(config),
            manifest,
        }
    }
}

#[derive(Debug, clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// In silent mode the wizard will install the Application silently, without any user interaction.
    #[arg(short, long, default_value_t = false)]
    silent: bool,
    /// Path to install the Application to in silent mode.
    #[arg(short = 'p', long, default_value = None)]
    install_path: Option<PathBuf>,
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

    fn get_manifest(&self) -> AppManifest {
        self.manifest.clone()
    }

    fn unattended_install(&mut self) -> Option<InstallConfig> {
        let args = Args::parse();

        if args.silent {
            let mut config = self.config.take()?;

            if let Some(path) = args.install_path {
                config.install_path = path;
            }

            Some(config)
        } else {
            None
        }
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
                    if let Some(config) = self.config.as_mut() {
                        config.install_path = path;
                        self.install_path_display = config.install_path.display().to_string();
                    }
                }
                WizardAction::None
            }

            Message::Back => WizardAction::Back,
            Message::Next => {
                if let Some(config) = self.config.take() {
                    WizardAction::Install(config)
                } else {
                    WizardAction::None
                }
            }
        }
    }

    fn view(&self) -> iced::Element<Self::Message> {
        Scaffold::new()
            .title(row![
                text(&self.manifest.name).size(24),
                horizontal_space(),
                text(&self.manifest.version).size(24)
            ])
            .control(text("Select install location").size(20))
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
