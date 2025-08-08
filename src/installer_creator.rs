#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::{
    fs::File,
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{config::AppManifest, post_embed::append_data};

#[derive(Debug, Error)]
pub enum CreateInstallerError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Cargo.toml error: {0}")]
    CargoTomlError(#[from] cargo_toml::Error),
    #[error("Zip error: {0}")]
    ZipError(#[from] zip::result::ZipError),
    #[error("Missing binary name in Cargo.toml")]
    MissingBinaryName,
    #[error("Config encode error: Postcard error: {0}")]
    PostcardError(#[from] postcard::Error),
    #[error("Couldn't find compiled binary")]
    BinaryMissing,
    #[error("Error while compiling binary")]
    CompileError,
}

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Create a new frost_wizard from a Cargo.toml
    Cargo {
        /// Filename of the resulting installer
        #[arg(short = 'o', long = "out")]
        installer_name: Option<PathBuf>,
        /// Path to the Cargo.toml
        #[arg(short = 'm', long = "manifest", default_value = "./Cargo.toml")]
        cargo_manifest_path: PathBuf,
    },
}

#[derive(Deserialize, Clone, Debug)]
struct Metadata {
    frost_wizard: Option<WizardMetadata>,
}
#[derive(Deserialize, Clone, Debug)]
struct WizardMetadata {
    friendly_name: Option<String>,
}

impl Metadata {
    fn friendly_name(&self) -> Option<String> {
        self.frost_wizard.as_ref()?.friendly_name.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    pub manifest: AppManifest,
    pub unpacked_size: u64,
}

pub fn create_installer() -> Result<(), CreateInstallerError> {
    let args = Args::parse();

    match args.command {
        Command::Cargo {
            installer_name,
            cargo_manifest_path,
        } => {
            let cargo_manifest =
                cargo_toml::Manifest::<Metadata>::from_path_with_metadata(&cargo_manifest_path)?;

            let bin = match cargo_manifest.bin.len() {
                0 => {
                    eprintln!("No binary found in Cargo.toml");
                    std::process::exit(1);
                }
                1 => cargo_manifest.bin[0].clone(),
                _ => {
                    eprintln!("More than one binary found in Cargo.toml");
                    std::process::exit(1);
                }
            };

            let bin_name = bin.name.ok_or(CreateInstallerError::MissingBinaryName)?;
            let installer_name = installer_name.unwrap_or_else(|| {
                #[cfg(windows)]
                return PathBuf::from(format!("{}_installer.exe", bin_name));
                #[cfg(not(windows))]
                return PathBuf::from(format!("{}_installer", bin_name));
            });
            let bin_name = format!("{}.exe", bin_name);

            println!("building binary with cargo...");

            let compile_status = std::process::Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(&cargo_manifest_path)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();

            match compile_status {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Failed to build binary");
                        return Err(CreateInstallerError::CompileError);
                    } else {
                        println!("Binary built successfully\nCreating installer...");
                    }
                }
                Err(err) => {
                    return Err(CreateInstallerError::IoError(err));
                }
            }

            let metadata = &cargo_manifest.package().metadata;
            let version = cargo_manifest.package().version();
            let friendly_name = metadata
                .as_ref()
                .map(Metadata::friendly_name)
                .flatten()
                .unwrap_or(bin_name.clone());

            let mut search_path = cargo_manifest_path
                .parent()
                .expect("Cargo.toml should have a parent");
            if search_path == Path::new("") {
                search_path = Path::new(".");
            }

            println!("Building installer for:\n{}\n{}", friendly_name, version);

            let mut bin_path = None;

            println!("Looking for compiled binary");

            while let Some(dir) = search_path.parent() {
                let mut temp_path = dir.to_path_buf();
                temp_path.push("target");
                temp_path.push("release");
                temp_path.push(bin_name.as_str());
                if temp_path.exists() {
                    bin_path = Some(temp_path);
                    break;
                }
                search_path = dir;
            }
            let bin_path = bin_path.ok_or(CreateInstallerError::BinaryMissing)?;

            println!("Found binary at {}", bin_path.display());

            let bin_file = File::open(bin_path)?;
            #[cfg(unix)]
            let bin_size = bin_file.metadata()?.size();
            #[cfg(windows)]
            let bin_size = bin_file.metadata()?.file_size();

            let embedded_config = EmbeddedConfig {
                manifest: AppManifest::build()
                    .friendly_name(friendly_name)
                    .bin_name(bin_name.clone())
                    .version(version),
                unpacked_size: bin_size,
            };

            let config_bytes = postcard::to_stdvec(&embedded_config)?;

            println!("Embedding Config");

            let mut append_writer = append_data(installer_name.as_ref())?;
            let length_bytes = (config_bytes.len() as u64).to_le_bytes();
            append_writer.write_all(&length_bytes)?;
            append_writer.write_all(&config_bytes)?;
            append_writer.move_start_to_current()?;

            println!("Zipping and embedding files");

            let mut zip = ZipWriter::new(append_writer);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Xz)
                .compression_level(Some(6i64));
            zip.start_file(bin_name.clone(), options)?;
            let mut bin_reader = BufReader::new(bin_file);
            std::io::copy(&mut bin_reader, &mut zip)?;

            println!("Flushing data");

            let mut append_writer = zip.finish()?;
            append_writer.flush()?;

            println!("Installer saved to {}", installer_name.display());

            Ok(())
        }
    }
}
