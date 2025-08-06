/// This module provides functionality for embedding data into the current executable by copying it and appending the data.
///
/// It also provides the function required to read the embedded data again.
use std::{
    env::current_exe,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use macros::hex_bytes;

// This is just some random data so the executable can check if it already contains data.
const FINGERPRINT: &[u8] =
    hex_bytes!("f4ac2a400195627734eb81b1cd2fe7019359dae01b7a8d40786beb164c580156");

pub fn search_for_embedded_data() -> Result<Option<EmbeddedReader>, io::Error> {
    let path = current_exe()?;
    let mut file = std::fs::File::open(path.as_path())?;
    file.seek(SeekFrom::End(-(FINGERPRINT.len() as i64)))?;
    let mut fprint = vec![0u8; FINGERPRINT.len()];
    file.read_exact(&mut fprint)?;

    if &fprint != FINGERPRINT {
        return Ok(None);
    }

    file.seek(SeekFrom::End(-(FINGERPRINT.len() as i64) - 8))?;
    let mut length_bytes = [0u8; 8];
    file.read_exact(&mut length_bytes)?;
    let length = u64::from_le_bytes(length_bytes);

    let seek_position = SeekFrom::End(-(FINGERPRINT.len() as i64) - 8 - length as i64);

    let start = file.seek(seek_position)?;

    let mut reader = EmbeddedReader::new(file, start, length);

    // let end = reader.seek(SeekFrom::End(0))?;
    // assert_eq!(length, end, "Error in seek implementation end");
    let start_pos = reader.seek(SeekFrom::Start(0))?;
    assert_eq!(start_pos, 0, "Error in seek implementation start");

    Ok(Some(reader))
}

#[derive(Debug)]
pub struct EmbeddedReader {
    file: File,
    start: u64,
    end: u64,
    position: u64,
}

impl EmbeddedReader {
    pub fn new(file: File, start: u64, length: u64) -> Self {
        let end = start + length;

        EmbeddedReader {
            file,
            start,
            end,
            position: start,
        }
    }

    pub fn move_start_to_current(&mut self) {
        self.start = self.position;
    }
}

impl Read for EmbeddedReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position == self.end {
            return Ok(0);
        }

        let max = std::cmp::min(buf.len() as u64, self.end - self.position) as usize;
        let n = self.file.read(&mut buf[..max])?;
        assert!(
            n as u64 <= self.end - self.position,
            "number of read bytes exceeds limit"
        );
        self.position += n as u64;
        Ok(n)
    }
}

impl Seek for EmbeddedReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => {
                let max = std::cmp::min(pos + self.start, self.end);
                self.file.seek(SeekFrom::Start(max))?
            }
            SeekFrom::End(pos) => {
                let pos = (self.end as i64 - pos) as u64;
                let max = std::cmp::min(pos, self.end);
                let clamped = std::cmp::max(max, self.start);
                self.file.seek(SeekFrom::Start(clamped))?
            }
            SeekFrom::Current(pos) => {
                if self.position as i64 + pos > self.end as i64 {
                    self.file.seek(SeekFrom::Start(self.end))?
                } else if self.position as i64 + pos < self.start as i64 {
                    self.file.seek(SeekFrom::Start(self.start))?
                } else {
                    self.file.seek(SeekFrom::Current(pos))?
                }
            }
        };

        self.position = new_pos;

        let corrected_position = new_pos - self.start;

        Ok(corrected_position)
    }
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
    initial_start: u64,
    start: u64,
    file: std::fs::File,
    flushed: bool,
}

impl AppendDataWriter {
    pub fn new(mut file: std::fs::File) -> Result<Self, std::io::Error> {
        let start = file.seek(SeekFrom::End(0))?;
        Ok(Self {
            initial_start: start,
            start,
            file,
            flushed: false,
        })
    }

    pub fn move_start_to_current(&mut self) -> Result<(), std::io::Error> {
        let new_start = self.file.seek(SeekFrom::Current(0))?;
        self.start = new_start;
        Ok(())
    }
}

impl Write for AppendDataWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.flushed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Already Flushed",
            ));
        }
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.flushed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Already Flushed",
            ));
        }
        self.flushed = true;
        let total_size = self.file.seek(SeekFrom::End(0))?;
        let written = total_size - self.initial_start;
        let length_bytes = written.to_le_bytes();
        self.file.write_all(&length_bytes)?;
        self.file.write_all(FINGERPRINT)?;

        self.file.sync_all()?;

        Ok(())
    }
}

impl Seek for AppendDataWriter {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(pos) => {
                let pos = pos + self.start;
                self.file.seek(SeekFrom::Start(pos))?
            }
            SeekFrom::End(pos) => self.file.seek(SeekFrom::End(pos))?,
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
