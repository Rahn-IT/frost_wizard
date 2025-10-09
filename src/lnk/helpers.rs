use byteorder::{BE, LE, ReadBytesExt};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
use std::{
    fmt::Debug,
    io::{self, Read},
};

pub fn read_u8(data: &mut impl Read) -> io::Result<u8> {
    data.read_u8()
}

#[must_use]
pub fn read_u16(data: &mut impl Read) -> io::Result<u16> {
    data.read_u16::<LE>()
}

#[must_use]
pub fn read_u32(data: &mut impl Read) -> io::Result<u32> {
    data.read_u32::<LE>()
}

#[must_use]
pub fn read_i32(data: &mut impl Read) -> io::Result<i32> {
    data.read_i32::<LE>()
}

#[must_use]
pub fn read_u64(data: &mut impl Read) -> io::Result<u64> {
    data.read_u64::<LE>()
}

const WINDOWS_EPOCH: u64 = 11644473600;

#[derive(Debug, thiserror::Error)]
pub enum WindowsDateTimeError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid Windows timestamp: {0}")]
    InvalidTimestamp(u64),
}

#[must_use]
pub fn read_windows_datetime(data: &mut impl Read) -> Result<NaiveDateTime, WindowsDateTimeError> {
    let windows_timestamp = read_u64(data)?;
    let unix_timestamp = (windows_timestamp / 10_000_000).saturating_sub(WINDOWS_EPOCH);

    let datetime = DateTime::from_timestamp(unix_timestamp as i64, 0)
        .ok_or_else(|| WindowsDateTimeError::InvalidTimestamp(windows_timestamp))?;

    Ok(datetime.naive_utc())
}

#[derive(Debug, thiserror::Error)]
pub enum StringReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("UTF-16 error: {0}")]
    Utf16Error(#[from] std::string::FromUtf16Error),
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

pub fn read_sized_string(data: &mut impl Read, utf16: bool) -> Result<String, StringReadError> {
    if utf16 {
        read_sized_utf16(data)
    } else {
        read_sized_utf8(data)
    }
}

#[must_use]
pub fn read_sized_utf16(data: &mut impl Read) -> Result<String, StringReadError> {
    let size = read_u16(data)? as usize;
    let mut raw_string = vec![0u8; size * 2];
    data.read_exact(&mut raw_string)?;
    let mut iter = raw_string.into_iter();
    let mut utf16 = Vec::with_capacity(size);
    while let Some((byte1, byte2)) = iter.next().zip(iter.next()) {
        let short = u16::from_le_bytes([byte1, byte2]);
        utf16.push(short);
    }

    Ok(String::from_utf16(&utf16)?)
}

#[must_use]
pub fn read_c_utf16(data: &mut impl Read) -> Result<String, StringReadError> {
    let mut encoded_string = Vec::new();
    loop {
        let short = read_u16(data)?;
        if short == 0 {
            break;
        }
        encoded_string.push(short);
    }

    let decoded_string = String::from_utf16(&encoded_string)?;
    Ok(decoded_string)
}

#[must_use]
pub fn read_sized_utf8(data: &mut impl Read) -> Result<String, StringReadError> {
    let size = read_u16(data)?;
    let mut raw_string = vec![0u8; size as usize];
    data.read_exact(&mut raw_string)?;
    Ok(String::from_utf8(raw_string)?)
}

#[must_use]
pub fn read_c_utf8(data: &mut impl Read, padding: bool) -> Result<String, StringReadError> {
    let mut encoded_string = Vec::new();
    loop {
        let byte = read_u8(data)?;
        if byte == 0 {
            break;
        }
        encoded_string.push(byte);
    }

    if padding && encoded_string.len() % 2 == 0 {
        let _padding = read_u8(data)?;
    }

    let decoded_string = String::from_utf8(encoded_string)?;
    Ok(decoded_string)
}

fn get_bits(short: u16, start: u8, length: u8) -> u16 {
    let mask = (1 << length) - 1;
    let shifted = short >> start;
    let result = shifted & mask;
    result
}

#[derive(Debug, thiserror::Error)]
pub enum DosDateTimeReadError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),
    #[error("Invalid DOS date: {0}-{1}-{2}")]
    InvalidDosDate(u16, u16, u16),
    #[error("Invalid DOS time: {0}:{1}:{2}")]
    InvalidDosTime(u16, u16, u16),
}

pub fn read_dos_datetime(data: &mut impl Read) -> Result<NaiveDateTime, DosDateTimeReadError> {
    let date = read_u16(data)?;
    let time = read_u16(data)?;
    let year = get_bits(date, 9, 7) + 1980;
    let month = get_bits(date, 5, 4).max(1);
    let day = get_bits(date, 0, 5).max(1);
    let hour = get_bits(time, 11, 5);
    let minute = get_bits(time, 5, 6);
    let second = get_bits(time, 0, 5);

    let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
        .ok_or_else(|| DosDateTimeReadError::InvalidDosDate(year, month, day))?;

    let time = NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
        .ok_or_else(|| DosDateTimeReadError::InvalidDosTime(hour, minute, second))?;

    Ok(NaiveDateTime::new(date, time))
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Debug for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

impl ToString for Guid {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

pub fn read_guid(data: &mut impl Read) -> Result<Guid, io::Error> {
    let data1 = read_u32(data)?;
    let data2 = read_u16(data)?;
    let data3 = read_u16(data)?;
    let mut data4 = [0u8; 8];
    data.read_exact(&mut data4)?;

    Ok(Guid {
        data1,
        data2,
        data3,
        data4,
    })
}
