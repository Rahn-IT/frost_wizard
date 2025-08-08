use std::path::PathBuf;

use clap::Parser;
use iced::{
    Alignment::Center,
    Task,
    widget::{button, checkbox, horizontal_space, row, text},
};
use rfd::AsyncFileDialog;

use crate::{
    config::{AppManifest, InstallConfig},
    ui::scaffold::Scaffold,
    wizard::{
        Wizard, WizardAction,
        basic::config::{BasicWizardBuilder, BasicWizardConfig},
    },
};

mod config;

enum Step {
    SelectInstallPath,
    SetInstallOptions,
}

pub struct BasicWizard {
    config: Option<InstallConfig>,
    selecting_path: bool,
    step: Step,
    manifest: AppManifest,
}

impl BasicWizard {
    pub fn builder() -> BasicWizardBuilder<(), ()> {
        BasicWizardConfig::build()
    }

    fn from_config(config: InstallConfig, manifest: AppManifest) -> Self {
        BasicWizard {
            config: Some(config),
            selecting_path: false,
            step: Step::SelectInstallPath,
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
    StartMenuShortcut(bool),
    DesktopShortcut(bool),
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
                self.selecting_path = true;
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
                self.selecting_path = false;
                if let Some(path) = path {
                    if let Some(config) = self.config.as_mut() {
                        config.install_path = path;
                    }
                }
                WizardAction::None
            }
            Message::StartMenuShortcut(create_shortcut) => {
                if let Some(config) = self.config.as_mut() {
                    config.create_desktop_shortcut = create_shortcut;
                }
                WizardAction::None
            }
            Message::DesktopShortcut(create_shortcut) => {
                if let Some(config) = self.config.as_mut() {
                    config.create_start_menu_shortcut = create_shortcut;
                }
                WizardAction::None
            }
            Message::Back => match self.step {
                Step::SelectInstallPath => WizardAction::Back,
                Step::SetInstallOptions => {
                    self.step = Step::SelectInstallPath;
                    WizardAction::None
                }
            },
            Message::Next => match self.step {
                Step::SelectInstallPath => {
                    self.step = Step::SetInstallOptions;
                    WizardAction::None
                }
                Step::SetInstallOptions => {
                    if let Some(config) = self.config.take() {
                        WizardAction::Install(config)
                    } else {
                        WizardAction::None
                    }
                }
            },
        }
    }

    fn view(&self) -> iced::Element<Self::Message> {
        let config = self.config.as_ref().unwrap();
        match self.step {
            Step::SelectInstallPath => Scaffold::new()
                .title(row![
                    text(&self.manifest.friendly_name).size(24),
                    horizontal_space(),
                    text(&self.manifest.version).size(24)
                ])
                .control(text("Select install location").size(20))
                .control(
                    row![
                        button("Select Folder").on_press(Message::SelectInstallPath),
                        if self.selecting_path {
                            text("Selecting...")
                        } else {
                            text(config.install_path.display().to_string())
                        }
                    ]
                    .spacing(20)
                    .align_y(Center),
                )
                .on_next_maybe((!self.selecting_path).then(|| Message::Next))
                .on_back(Message::Back)
                .into(),
            Step::SetInstallOptions => Scaffold::new()
                .title(row![
                    text(&self.manifest.friendly_name).size(24),
                    horizontal_space(),
                    text(&self.manifest.version).size(24)
                ])
                .control(text("Set installation options").size(20))
                .control(
                    checkbox(
                        "Create start menu shortcut",
                        config.create_start_menu_shortcut,
                    )
                    .on_toggle(Message::StartMenuShortcut),
                )
                .control(
                    checkbox("Create desktop shortcut", config.create_desktop_shortcut)
                        .on_toggle(Message::StartMenuShortcut),
                )
                .on_next_maybe((!self.selecting_path).then(|| Message::Next))
                .on_back(Message::Back)
                .into(),
        }
    }
}
