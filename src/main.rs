use std::{
    fs::File,
    io::{BufReader, Read, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use clap::Parser;
use frost_wizard::{
    AppManifest, BasicWizard,
    post_embed::{append_data, search_for_embedded_data},
};
use serde::{Deserialize, Serialize};
use zip::{ZipWriter, write::SimpleFileOptions};

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Create a new frost_wizard from the current rust crate
    Create {
        /// Filename of the resulting installer
        #[arg(short = 'o', long = "out")]
        installer_name: Option<PathBuf>,
        /// Path to the Cargo.toml
        #[arg(short, long, default_value = "Cargo.toml")]
        manifest_path: PathBuf,
    },
}

#[derive(Deserialize, Clone, Debug)]
struct Metadata {
    frost_wizard: Option<WizardMetadata>,
}
#[derive(Deserialize, Clone, Debug)]
struct WizardMetadata {
    friendly_name: Option<String>,
    default_install_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    pub default_install_path: PathBuf,
    pub manifest: AppManifest,
    pub unpacked_size: u64,
}

fn main() {
    if let Some(mut embedded_reader) =
        search_for_embedded_data().expect("Error while checking for embedded data")
    {
        let mut len_bytes = [0u8; 8];
        embedded_reader.read_exact(&mut len_bytes).unwrap();
        let manifest_len = u64::from_le_bytes(len_bytes);

        let mut config_bytes = vec![0u8; manifest_len as usize];
        embedded_reader.read_exact(&mut config_bytes).unwrap();
        let config: EmbeddedConfig = postcard::from_bytes(&config_bytes).unwrap();

        embedded_reader.move_start_to_current();

        BasicWizard::builder()
            .manifest(config.manifest)
            .default_install_path(config.default_install_path)
            .add_payload(frost_wizard::FilePayload::Directory {
                unpacked_size: config.unpacked_size,
                reader: Box::new(embedded_reader),
            })
            .to_installer()
            .run()
            .unwrap();

        std::process::exit(0);
    }

    let args = Args::parse();

    match args.command {
        Command::Create {
            installer_name,
            manifest_path,
        } => {
            let cargo_manifest =
                cargo_toml::Manifest::<Metadata>::from_path_with_metadata(&manifest_path).unwrap();

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

            let bin_name = bin.name.unwrap();
            let installer_name = installer_name.unwrap_or_else(|| {
                #[cfg(target_os = "windows")]
                return PathBuf::from(format!("{}_installer.exe", bin_name));
                #[cfg(not(target_os = "windows"))]
                return PathBuf::from(format!("{}_installer", bin_name));
            });

            println!("building binary with cargo...");

            let compile_status = std::process::Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--manifest-path")
                .arg(&manifest_path)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();

            match compile_status {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Failed to build binary");
                        std::process::exit(1);
                    } else {
                        println!("Binary built successfully\nCreating installer...");
                    }
                }
                Err(err) => {
                    eprintln!("Error while building binary: {}", err);
                    std::process::exit(1);
                }
            }

            let metadata = &cargo_manifest.package().metadata;
            let version = cargo_manifest.package().version();
            let friendly_name = metadata
                .as_ref()
                .map(Metadata::friendly_name)
                .flatten()
                .unwrap_or(bin_name.clone());
            let default_install_path = metadata
                .as_ref()
                .map(Metadata::default_install_path)
                .flatten()
                // TODO: generate install path from bin name
                .unwrap_or_else(|| PathBuf::from("/home/acul/test"));

            let mut search_path = manifest_path.parent().unwrap();
            if search_path == Path::new("") {
                search_path = Path::new(".");
            }

            let mut bin_path = None;

            while let Some(dir) = search_path.parent() {
                let mut search_path = dir.to_path_buf();
                search_path.push("target");
                search_path.push("release");
                search_path.push(bin_name.as_str());
                if search_path.exists() {
                    bin_path = Some(search_path);
                    break;
                }
            }
            let bin_path = bin_path.expect("Couldn't find compiled binary");

            let bin_file = File::open(bin_path).unwrap();
            let bin_size = bin_file.metadata().unwrap().size();

            let embedded_config = EmbeddedConfig {
                default_install_path,
                manifest: AppManifest::build().name(friendly_name).version(version),
                unpacked_size: bin_size,
            };

            let config_bytes = postcard::to_stdvec(&embedded_config).unwrap();

            let mut append_writer = append_data(installer_name.as_ref()).unwrap();
            let length_bytes = (config_bytes.len() as u64).to_le_bytes();
            append_writer.write_all(&length_bytes).unwrap();
            append_writer.write_all(&config_bytes).unwrap();
            append_writer.move_start_to_current().unwrap();

            let mut zip = ZipWriter::new(append_writer);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Xz)
                .compression_level(Some(6i64));
            zip.start_file(bin_name.clone(), options).unwrap();
            let mut bin_reader = BufReader::new(bin_file);
            std::io::copy(&mut bin_reader, &mut zip).unwrap();

            let mut append_writer = zip.finish().unwrap();
            append_writer.flush().unwrap();

            println!("Installer saved to {}", installer_name.display());
        }
    }
}

impl Metadata {
    fn friendly_name(&self) -> Option<String> {
        self.frost_wizard.as_ref()?.friendly_name.clone()
    }

    fn default_install_path(&self) -> Option<PathBuf> {
        self.frost_wizard.as_ref()?.default_install_path.clone()
    }
}
