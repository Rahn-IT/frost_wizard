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
        // .add_payload({
        //     let data = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        //     FilePayload::Directory {
        //         reader: Box::new(std::io::Cursor::new(data)),
        //         unpacked_size: 10,
        //     }
        // })
        .to_installer()
        .run()
        .unwrap();
}
