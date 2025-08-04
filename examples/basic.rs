use frost_wizard::{AppManifest, BasicWizard, FilePayload, embed_directory};

fn main() {
    BasicWizard::builder()
        .default_install_path("/home/acul/test")
        .manifest(
            AppManifest::build()
                .name("test")
                .version("0.1.0")
                .publisher("Rahn-IT"),
        )
        .add_payload(embed_directory!("testdata"))
        .to_installer()
        .run()
        .unwrap();
}
