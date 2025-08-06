#[cfg(target_os = "windows")]
use std::path::Path;
use std::{
    fs,
    io::{Read, Write},
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};
use sipper::{FutureExt, Sipper, sipper};
use tokio::sync::mpsc;
use zip::{ZipArchive, result::ZipError};

use crate::{
    config::{AppManifest, FilePayload, InstallConfig},
    ui::InstallerUi,
};

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

    pub fn run(mut self) -> Result<(), iced::Error> {
        if let Some(config) = self.wizard.unattended_install() {
            // Perform unattended installation using the provided config
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let name = self.manifest.name.clone();
            let install_result =
                runtime.block_on(async { install_unattended(config, self.manifest).await });
            match install_result {
                Ok(()) => {
                    println!("{} installed successfully!", name);
                    std::process::exit(0);
                }
                Err(err) => {
                    eprintln!("Error during unattended install: {}", err);
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
            .expect("Fixed template can't fail")
            .progress_chars("##-"),
        )
        .with_message(format!("Installing {}", manifest.name));

    bar.enable_steady_tick(Duration::from_millis(100));

    while let Some(progress) = sipper.sip().await {
        bar.set_position((progress * BAR_FACTOR) as u64);
    }

    let result = sipper.await;

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
    #[cfg(windows)]
    #[error("Failed to set Registry Keys:\n{0}")]
    RegistryError(windows_result::Error)
}

pub(crate) fn install<Output>(
    config: InstallConfig,
    manifest: AppManifest,
    mapper: impl Fn(Result<(), InstallError>) -> Output,
) -> impl sipper::Sipper<Output, f32> {
    let sipper = sipper(|mut sender| {
        async move {
            let (send, mut recv) = mpsc::channel(100);

            let install_future = inner_install(send, config, manifest);

            tokio::spawn(async move {
                while let Some(progress) = recv.recv().await {
                    sender.send(progress).await;
                }
            });

            install_future.await
        }
        .map(mapper)
    });

    sipper
}

async fn inner_install(
    sender: mpsc::Sender<f32>,
    config: InstallConfig,
    _manifest: AppManifest,
) -> Result<(), InstallError> {
    tokio::task::spawn_blocking(move || {
        fs::create_dir_all(&config.install_path).map_err(InstallError::CreateInstallDir)?;

        let mut full_size = 0u64;

        // Calculate overall size

        for payload in &config.payloads {
            match payload {
                FilePayload::File { contents, .. } => full_size += contents.len() as u64,
                FilePayload::Directory { unpacked_size, .. } => {
                    full_size += *unpacked_size;
                }
            }
        }

        let _full_size_kb = full_size / 1024;
        // Size is a bit larger, so the progress isn't yet full when adding services and creating the registry entries
        let full_size = full_size as f32 * 1.1;

        let mut written = 0u64;

        for payload in config.payloads {
            match payload {
                FilePayload::File { name, contents } => {
                    let path = config.install_path.join(name.as_ref());
                    fs::write(path, &contents).map_err(InstallError::WritePayload)?;

                    written += contents.len() as u64;

                    sender.blocking_send(written as f32 / full_size).unwrap();
                }
                FilePayload::Directory { reader, .. } => {
                    let mut zip = ZipArchive::new(reader).map_err(InstallError::ZipError)?;

                    for index in 0..zip.len() {
                        let mut reader = zip.by_index(index).map_err(InstallError::ZipError)?;
                        let path = config.install_path.join(reader.name());
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(parent).map_err(InstallError::WritePayload)?;
                        }
                        let mut file =
                            fs::File::create(path).map_err(InstallError::WritePayload)?;

                        let mut buf = [0; 8192];

                        loop {
                            let n = reader.read(&mut buf).map_err(InstallError::WritePayload)?;
                            if n == 0 {
                                break;
                            }

                            file.write_all(&buf[..n])
                                .map_err(InstallError::WritePayload)?;

                            written += n as u64;

                            sender.blocking_send(written as f32 / full_size).unwrap();
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            set_registry_keys(&_manifest, &config.install_path, written).map_err(InstallError::RegistryError)?;
        }

        sender.blocking_send(1.0).unwrap();

        Ok(())
    })
    .await
    .unwrap()
}

#[cfg(target_os = "windows")]
fn set_registry_keys(manifest: &AppManifest, install_location: &Path, size: u64) -> Result<(), windows_result::Error> {
    let name_for_path = manifest.name.replace(|c: char| !c.is_alphanumeric(), "");
    let registry_path = format!(
        "\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
        name_for_path
    );
    let key = windows_registry::LOCAL_MACHINE.create(&registry_path)?;
    key.set_string("DisplayName", &manifest.name)?;
    key.set_string("DisplayVersion", &manifest.version)?;
    key.set_string("InstallLocation", install_location.to_string_lossy().as_ref())?;
    key.set_u32("EstimatedSize", size as u32)?;

    if let Some(publisher) = &manifest.publisher {
        key.set_string("Publisher", publisher)?;
    }

    Ok(())
}
