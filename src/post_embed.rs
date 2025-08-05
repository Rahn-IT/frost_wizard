use std::{
    env::current_exe,
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use macros::hex_bytes;
use zip::unstable::{LittleEndianReadExt, LittleEndianWriteExt};

// This is just some random data so the executable can check if it already contains data.
const FINGERPRINT: &[u8] =
    hex_bytes!("f4ac2a400195627734eb81b1cd2fe7019359dae01b7a8d40786beb164c580156");

pub fn search_for_embedded_data() -> Result<Option<impl Read>, io::Error> {
    let path = current_exe()?;
    let mut file = std::fs::File::open(path.as_path())?;
    file.seek(SeekFrom::End(-(FINGERPRINT.len() as i64)))?;
    let mut fprint = vec![0u8; FINGERPRINT.len()];
    file.read_exact(&mut fprint)?;

    if &fprint != FINGERPRINT {
        return Ok(None);
    }

    file.seek(SeekFrom::End(-(FINGERPRINT.len() as i64) - 8))?;
    let length = file.read_u64_le()?;
    println!("Found embedded data section with {} bytes", length);
    println!("Total file size: {}", file.metadata()?.len());
    let seek_position = SeekFrom::End(-(FINGERPRINT.len() as i64) - 8 - length as i64);

    println!("Seeking to position {:?}", seek_position);
    file.seek(seek_position)?;

    println!("Creating Take");
    let reader = file.take(length);

    Ok(Some(reader))
}

pub fn append_data(new_executable: &Path) -> Result<AppendDataWriter, std::io::Error> {
    let source = current_exe().unwrap();
    std::fs::copy(source, new_executable)?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(new_executable)?;

    let current_size = file.seek(SeekFrom::End(0))?;
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
        file.write_all(&zeros)?;
    }

    AppendDataWriter::new(file)
}

pub struct AppendDataWriter {
    start: u64,
    file: std::fs::File,
}

impl AppendDataWriter {
    pub fn new(mut file: std::fs::File) -> Result<Self, std::io::Error> {
        let start = file.seek(SeekFrom::End(0))?;
        Ok(Self { start, file })
    }
}

impl Write for AppendDataWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let total_size = self.file.seek(SeekFrom::End(0))?;
        println!("Total file size: {}", self.file.metadata()?.len());
        let written = total_size - self.start;
        println!("Finishing embedded data section with {} bytes", written);
        self.file.write_u64_le(written)?;
        self.file.write_all(FINGERPRINT)?;

        self.file.sync_all()?;

        Ok(())
    }
}

impl Seek for AppendDataWriter {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => self.file.seek(SeekFrom::Start(pos + self.start))?,
            SeekFrom::End(mut pos) => {
                let min = -(self.start as i64);
                if pos < min {
                    pos = min;
                }
                self.file.seek(SeekFrom::End(pos))?
            }
            SeekFrom::Current(pos) => {
                let new_pos = self.file.seek(SeekFrom::Current(pos))?;
                if new_pos < self.start {
                    self.file.seek(SeekFrom::Start(self.start))?;
                }
                new_pos
            }
        };

        Ok(new_pos - self.start)
    }
}
