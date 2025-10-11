use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::{
    fmt::Debug,
    io::{self, Read, Write},
};

use crate::lnk::LnkWriteError;

pub fn read_u8(data: &mut impl Read) -> io::Result<u8> {
    data.read_u8()
}

pub fn write_u8(data: &mut impl Write, value: u8) -> io::Result<()> {
    data.write_u8(value)
}

#[must_use]
pub fn read_u16(data: &mut impl Read) -> io::Result<u16> {
    data.read_u16::<LE>()
}

#[must_use]
pub fn write_u16(data: &mut impl Write, value: u16) -> io::Result<()> {
    data.write_u16::<LE>(value)
}

#[must_use]
pub fn read_u32(data: &mut impl Read) -> io::Result<u32> {
    data.read_u32::<LE>()
}

#[must_use]
pub fn write_u32(data: &mut impl Write, value: u32) -> io::Result<()> {
    data.write_u32::<LE>(value)
}

#[must_use]
pub fn read_i32(data: &mut impl Read) -> io::Result<i32> {
    data.read_i32::<LE>()
}

#[must_use]
pub fn write_i32(data: &mut impl Write, value: i32) -> io::Result<()> {
    data.write_i32::<LE>(value)
}

#[must_use]
pub fn read_u64(data: &mut impl Read) -> io::Result<u64> {
    data.read_u64::<LE>()
}

#[must_use]
pub fn write_u64(data: &mut impl Write, value: u64) -> io::Result<()> {
    data.write_u64::<LE>(value)
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

pub fn write_windows_datetime(data: &mut impl Write, datetime: NaiveDateTime) -> io::Result<()> {
    let unix_timestamp = datetime.and_utc().timestamp() as u64 + WINDOWS_EPOCH;
    let windows_timestamp = unix_timestamp * 10_000_000;

    write_u64(data, windows_timestamp)
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

pub fn write_sized_utf16(data: &mut impl Write, string: &str) -> Result<(), io::Error> {
    let size = string.chars().count() as u16;
    write_u16(data, size)?;
    write_c_utf16(data, string)?;
    Ok(())
}

#[must_use]
pub fn write_c_utf16(data: &mut impl Write, string: &str) -> Result<(), io::Error> {
    let mut encoded_string: Vec<u8> = string
        .encode_utf16()
        .flat_map(|short| short.to_le_bytes())
        .collect();
    encoded_string.push(0);
    encoded_string.push(0);

    data.write_all(&encoded_string)?;
    Ok(())
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

fn set_bits(short: &mut u16, value: u16, start: u8, length: u8) {
    let mask = (1 << length) - 1;
    let shifted = value << start;
    *short = *short & !(mask << start) | shifted;
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

pub fn write_dos_datetime(
    data: &mut impl Write,
    datetime: NaiveDateTime,
) -> Result<(), LnkWriteError> {
    let date = datetime.date();
    let time = datetime.time();

    let year = (date.year() as u32).saturating_sub(1980);
    let month = date.month();
    let day = date.day();

    let hour = time.hour();
    let minute = time.minute();
    let second = time.second();

    let mut date = 0u16;
    set_bits(&mut date, year as u16, 9, 7);
    set_bits(&mut date, month as u16, 5, 4);
    set_bits(&mut date, day as u16, 0, 5);

    let mut time = 0u16;
    set_bits(&mut time, hour as u16, 11, 5);
    set_bits(&mut time, minute as u16, 5, 6);
    set_bits(&mut time, second as u16, 0, 5);

    write_u16(data, date)?;
    write_u16(data, time)?;

    Ok(())
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

#[derive(Debug, thiserror::Error)]
pub enum GuidStringParseError {
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Invalid length")]
    InvalidLength,
    #[error("Invalid integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl Guid {
    pub fn from_str(s: &str) -> Result<Self, GuidStringParseError> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return Err(GuidStringParseError::InvalidFormat);
        }

        let data1 = u32::from_str_radix(parts[0], 16)?;
        let data2 = u16::from_str_radix(parts[1], 16)?;
        let data3 = u16::from_str_radix(parts[2], 16)?;

        let mut data4 = [0u8; 8];
        let mut iter = parts[3..].iter().flat_map(|part| part.chars());

        let mut index = 0;
        while let Some((byte1, byte2)) = iter.next().zip(iter.next()) {
            if index >= data4.len() {
                return Err(GuidStringParseError::InvalidLength);
            }
            let mut byte_string = String::new();
            byte_string.push(byte1);
            byte_string.push(byte2);
            data4[index] = u8::from_str_radix(&byte_string, 16)?;
            index += 1;
        }

        Ok(Guid {
            data1,
            data2,
            data3,
            data4,
        })
    }

    pub fn write(&self, data: &mut impl Write) -> Result<(), io::Error> {
        write_u32(data, self.data1)?;
        write_u16(data, self.data2)?;
        write_u16(data, self.data3)?;
        data.write_all(&self.data4)?;

        Ok(())
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
