use std::io::{self, Read};
use thiserror::Error;

use crate::lnk::helpers::{read_u16, read_u32};

#[derive(Debug, Error)]
pub enum ConsoleDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid block size: expected 0x000000CC, got 0x{0:08X}")]
    InvalidBlockSize(u32),
    #[error("invalid block signature: expected 0xA0000002, got 0x{0:08X}")]
    InvalidSignature(u32),
    #[error("invalid UTF-16 sequence in FaceName")]
    InvalidFaceName,
}

#[derive(Debug, Clone)]
pub struct ConsoleDataBlock {
    pub fill_attributes: u16,
    pub popup_fill_attributes: u16,
    pub screen_buffer_size_x: i16,
    pub screen_buffer_size_y: i16,
    pub window_size_x: i16,
    pub window_size_y: i16,
    pub window_origin_x: i16,
    pub window_origin_y: i16,
    pub font_size: u32,
    pub font_family: u32,
    pub font_weight: u32,
    pub face_name: String, // 32 UTF-16LE code units, NUL-padded
    pub cursor_size: u32,
    pub full_screen: u32,
    pub quick_edit: u32,
    pub insert_mode: u32,
    pub auto_position: u32,
    pub history_buffer_size: u32,
    pub number_of_history_buffers: u32,
    pub history_no_dup: u32,
    pub color_table: [u32; 16],
}

impl ConsoleDataBlock {
    pub fn parse(data: &mut impl Read) -> Result<Self, ConsoleDataBlockParseError> {
        // Block header
        let block_size = read_u32(data)?;
        if block_size != 0x0000_00CC {
            return Err(ConsoleDataBlockParseError::InvalidBlockSize(block_size));
        }
        let sig = read_u32(data)?;
        if sig != 0xA000_0002 {
            return Err(ConsoleDataBlockParseError::InvalidSignature(sig));
        }

        // Helpers for i16 as spec uses signed 16-bit in several places.
        fn read_i16_from_le(data: &mut impl Read) -> io::Result<i16> {
            let v = read_u16(data)?;
            Ok(i16::from_le_bytes(v.to_le_bytes()))
        }

        let fill_attributes = read_u16(data)?;
        let popup_fill_attributes = read_u16(data)?;
        let screen_buffer_size_x = read_i16_from_le(data)?;
        let screen_buffer_size_y = read_i16_from_le(data)?;
        let window_size_x = read_i16_from_le(data)?;
        let window_size_y = read_i16_from_le(data)?;
        let window_origin_x = read_i16_from_le(data)?;
        let window_origin_y = read_i16_from_le(data)?;

        // Unused1 / Unused2 (both u32) — must be read and ignored.
        let _unused1 = read_u32(data)?;
        let _unused2 = read_u32(data)?;

        let font_size = read_u32(data)?;
        let font_family = read_u32(data)?;
        let font_weight = read_u32(data)?;

        // Face Name: exactly 64 bytes = 32 UTF-16LE code units (NUL padded).
        let mut face_buf = [0u8; 64];
        data.read_exact(&mut face_buf)?;
        let mut u16s = [0u16; 32];
        for (i, chunk) in face_buf.chunks_exact(2).enumerate() {
            u16s[i] = u16::from_le_bytes([chunk[0], chunk[1]]);
        }
        let face_end = u16s.iter().position(|&c| c == 0).unwrap_or(u16s.len());
        let face_name = String::from_utf16(&u16s[..face_end])
            .map_err(|_| ConsoleDataBlockParseError::InvalidFaceName)?;

        let cursor_size = read_u32(data)?;
        let full_screen = read_u32(data)?;
        let quick_edit = read_u32(data)?;
        let insert_mode = read_u32(data)?;
        let auto_position = read_u32(data)?;
        let history_buffer_size = read_u32(data)?;
        let number_of_history_buffers = read_u32(data)?;
        let history_no_dup = read_u32(data)?;

        // Color table: 16 x u32 RGBA values (LE) as per spec (64 bytes total).
        let mut color_table = [0u32; 16];
        for i in 0..16 {
            color_table[i] = read_u32(data)?;
        }

        Ok(Self {
            fill_attributes,
            popup_fill_attributes,
            screen_buffer_size_x,
            screen_buffer_size_y,
            window_size_x,
            window_size_y,
            window_origin_x,
            window_origin_y,
            font_size,
            font_family,
            font_weight,
            face_name,
            cursor_size,
            full_screen,
            quick_edit,
            insert_mode,
            auto_position,
            history_buffer_size,
            number_of_history_buffers,
            history_no_dup,
            color_table,
        })
    }
}
