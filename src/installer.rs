use std::{io::Read, time::Duration};

use indicatif::{ProgressBar, ProgressStyle};
use sipper::{FutureExt, Sipper, sipper};
use tokio::{fs, io::AsyncWriteExt};
use zip::{ZipArchive, result::ZipError};

use crate::{AppManifest, FilePayload, config::InstallConfig, ui::InstallerUi};

pub mod basic;

pub struct Installer<Wizard> {
    manifest: AppManifest,
    wizard: Wizard,
}

impl<Wizard> Installer<Wizard>
where
    Wizard: crate::wizard::Wizard,
{
    pub fn from_wizard(wizard: Wizard) -> Self {
        let manifest = wizard.get_manifest();
        Self { wizard, manifest }
    }

    pub fn run(self) -> Result<(), iced::Error> {
        if let Some(config) = self.wizard.unattended_install() {
            // Perform unattended installation using the provided config
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let install_result =
                runtime.block_on(async { install_unattended(config, self.manifest).await });
            match install_result {
                Ok(()) => {
                    std::process::exit(0);
                }
                Err(_) => {
                    std::process::exit(1);
                }
            }
        } else {
            iced::application(
                InstallerUi::<Wizard>::title,
                InstallerUi::<Wizard>::update,
                InstallerUi::<Wizard>::view,
            )
            .run_with(move || InstallerUi::start(self.wizard, self.manifest))
        }
    }
}

const BAR_FACTOR: f32 = 1000.0;

async fn install_unattended(
    config: InstallConfig,
    manifest: AppManifest,
) -> Result<(), InstallError> {
    let mut sipper = install(config, manifest.clone(), |progress| progress).pin();

    let bar = ProgressBar::new(BAR_FACTOR as u64)
        .with_style(
            ProgressStyle::with_template(
                "{spinner} {msg}\n[{percent}%] {wide_bar:40.cyan/blue} [{elapsed}]",
            )
            .unwrap()
            .progress_chars("##-"),
        )
        .with_message(format!("Installing {}", manifest.name));

    bar.enable_steady_tick(Duration::from_millis(100));

    while let Some(progress) = sipper.sip().await {
        bar.set_position((progress * BAR_FACTOR) as u64);
    }

    let result = sipper.await;

    match &result {
        Ok(()) => bar.finish_with_message(format!("{} installed successfully!", manifest.name)),
        Err(err) => bar.finish_with_message(format!("Installation failed: {}", err)),
    }

    result
}

#[derive(Debug, thiserror::Error)]
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

pub(crate) fn install<Output>(
    config: InstallConfig,
    _manifest: AppManifest,
    mapper: impl Fn(Result<(), InstallError>) -> Output,
) -> impl sipper::Sipper<Output, f32> {
    let sipper = sipper(|mut sender| {
        async move {
            fs::create_dir_all(&config.install_path)
                .await
                .map_err(InstallError::CreateInstallDir)?;

            let mut full_size = 0u64;

            // Calculate overall size

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

            let _full_size_kb = full_size / 1024;
            // Size is a bit larger, so the progress isn't yet full when adding services and creating the registry entries
            let full_size = full_size as f32 * 1.1;

            let mut written = 0u64;

            for payload in &config.payloads {
                match payload.as_ref() {
                    FilePayload::File { name, contents } => {
                        let path = config.install_path.join(name.as_ref());
                        fs::write(path, contents)
                            .await
                            .map_err(InstallError::WritePayload)?;

                        written += contents.len() as u64;

                        sender.send(written as f32 / full_size).await;
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

                                sender.send(written as f32 / full_size).await;
                            }
                        }
                    }
                }
            }

            #[cfg(target_os = "windows")]
            {
                let name_for_path = _manifest.name.replace(|c: char| !c.is_alphanumeric(), "");
                let registry_path = format!(
                    "\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
                    name_for_path
                );
                let key = windows_registry::LOCAL_MACHINE::create(&path)?;
                key.set_string("DisplayName", _manifest.name)?;
                key.set_string("DisplayVersion", _manifest.version)?;
                key.set_string("InstallLocation", config.install_location)?;
                key.set_u32("EstimatedSize", full_size_kb as u32)?;

                if let Some(publisher) = _manifest.publisher {
                    key.set_string("Publisher", publisher)?;
                }
            }

            sender.send(1.0).await;

            Ok(())
        }
        .map(mapper)
    });

    sipper
}
