use frost_wizard::{config::AppManifest, embed_directory, wizard::basic::BasicWizard};

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
