use frost_wizard::{AppManifest, BasicWizard, FilePayload};

fn main() {
    BasicWizard::build()
        .default_install_path("/home/acul")
        .manifest(AppManifest::build().name("test").version("0.1.0"))
        .payload(FilePayload::File {
            name: "test.txt".into(),
            contents: b"Das ist ein Test!".into(),
        })
        .to_installer()
        .run()
        .unwrap();
}
