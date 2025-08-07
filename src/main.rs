#![windows_subsystem = "windows"]
use std::io::Read;
use thiserror::Error;

use frost_wizard::{
    config::FilePayload,
    installer_creator::{EmbeddedConfig, create_installer},
    post_embed::{EmbeddedReader, search_for_embedded_data},
    wizard::basic::BasicWizard,
};

use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

fn main() {
    let _attach_result = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    if let Some(embedded_reader) =
        search_for_embedded_data().expect("Error while checking for embedded data")
    {
        if let Err(err) = start_installer_from_embedded_data(embedded_reader) {
            eprintln!("Error while running installer: {}", err);
            std::process::exit(1);
        }
        std::process::exit(0);
    } else {
        if let Err(err) = create_installer() {
            eprintln!("Error creating installer: {}", err);
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Error)]
enum StartInstallerError {
    #[error("Error reading embedded data")]
    ReadError(#[from] std::io::Error),
    #[error("Error decoding embedded data")]
    PostError(#[from] postcard::Error),
    #[error("Error while running installer")]
    InstallerError(#[from] iced::Error),
}

fn start_installer_from_embedded_data(
    mut reader: EmbeddedReader,
) -> Result<(), StartInstallerError> {
    let mut len_bytes = [0u8; 8];
    reader.read_exact(&mut len_bytes)?;
    let manifest_len = u64::from_le_bytes(len_bytes);

    let mut config_bytes = vec![0u8; manifest_len as usize];
    reader.read_exact(&mut config_bytes)?;
    let config: EmbeddedConfig = postcard::from_bytes(&config_bytes)?;

    reader.move_start_to_current();

    BasicWizard::builder()
        .manifest(config.manifest)
        .default_install_path(config.default_install_path)
        .add_payload(FilePayload::Directory {
            unpacked_size: config.unpacked_size,
            reader: Box::new(reader),
        })
        .to_installer()
        .run()?;

    Ok(())
}
