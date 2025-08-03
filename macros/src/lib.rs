use proc_macro::TokenStream;
use quote::quote;
use std::path::Path;
use std::{fs, io};
use syn::{LitStr, parse_macro_input};
use zip::{ZipWriter, write::SimpleFileOptions};

/// Recursively collect all files in a directory
fn collect_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_files(&path, files)?;
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

#[proc_macro]
pub fn include_dir_zip(input: TokenStream) -> TokenStream {
    // Parse the input path
    let input_path = parse_macro_input!(input as LitStr).value();
    let input_path = Path::new(&input_path)
        .canonicalize()
        .expect("Failed to canonicalize path");

    // Check if path exists and is a directory
    if !input_path.is_dir() {
        panic!(
            "Path does not exist or is not a directory: {}",
            input_path.display()
        );
    }

    // Collect all files in the directory
    let mut files = Vec::new();
    collect_files(input_path.as_path(), &mut files).unwrap_or_else(|e| {
        panic!("Failed to read directory {}: {}", input_path.display(), e);
    });

    let mut buf = Vec::new();

    let mut zip_writer = ZipWriter::new(std::io::Cursor::new(&mut buf));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Xz)
        .compression_level(Some(6i64));

    // Generate the output structure
    for path in files {
        let zip_path = path
            .strip_prefix(&input_path)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");

        zip_writer
            .start_file_from_path(zip_path, options)
            .expect("Couldn't create file in archive");

        let mut file = fs::File::open(path).expect("Failed to open file");

        io::copy(&mut file, &mut zip_writer).expect("Failed to copy file to zip");
    }

    zip_writer.finish().expect("Failed to finish zip writer");

    let expanded = quote! {
            // Directory structure with file contents
            &[#(#buf),*]
    };

    TokenStream::from(expanded)
}
