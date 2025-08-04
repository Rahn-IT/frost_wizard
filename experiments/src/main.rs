use std::{
    env::current_exe,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    num::ParseIntError,
    path::PathBuf,
};

use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Inject,
    Read,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Inject => {
            append_data(b"Hello world!");
        }
        Command::Read => {
            let data = read_data();
            let string = String::from_utf8_lossy(&data);
            println!("{:?}", string);
        }
    }

    println!("{:?}", args);
}

fn fingerprint() -> Vec<u8> {
    decode_hex("f4ac2a400195627734eb81b1cd2fe7019359dae01b7a8d40786beb164c580156").unwrap()
}

fn append_data(data: &[u8]) {
    let path = current_exe().unwrap();
    fs::copy(path, "./test").unwrap();
    let path = PathBuf::from("./test");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();

    let fingerprint = fingerprint();
    let length_bytes = (data.len() as u64).to_le_bytes();

    let current_size = file.seek(SeekFrom::End(0)).unwrap();
    // let new_size =
    //     current_size + data.len() as u64 + fingerprint.len() as u64 + length_bytes.len() as u64;

    let alignment = 4096;
    let misalignment = current_size % alignment;
    let padding_size = if misalignment != 0 {
        alignment - misalignment
    } else {
        0
    };

    if padding_size > 0 {
        let zeros = vec![0u8; padding_size as usize];
        file.write_all(&zeros).unwrap();
    }

    file.write_all(data).unwrap();
    file.write_all(&length_bytes).unwrap();
    file.write_all(&fingerprint).unwrap();
}

fn read_data() -> Vec<u8> {
    let path = current_exe().unwrap();
    let mut file = OpenOptions::new().read(true).open(path).unwrap();
    file.seek(SeekFrom::End(-32)).unwrap();
    let mut fprint = vec![0u8; 32];
    file.read_exact(&mut fprint).unwrap();

    if &fprint != &fingerprint() {
        panic!("Fingerprint mismatch! - No data included")
    }

    file.seek(SeekFrom::End(-32 - 8)).unwrap();
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).unwrap();
    let length = u64::from_le_bytes(buf);

    let mut data = vec![0u8; length as usize];
    file.seek(SeekFrom::End(-32 - 8 - length as i64)).unwrap();
    file.read_exact(&mut data).unwrap();

    data
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
