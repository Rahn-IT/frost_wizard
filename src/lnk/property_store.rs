use chrono::NaiveDateTime;
use std::{
    collections::BTreeMap,
    io::{self, Cursor, Read},
};
use thiserror::Error;

use crate::lnk::{
    GUID,
    helpers::{
        Guid, StringReadError, WindowsDateTimeError, read_c_utf16, read_guid, read_u8, read_u16,
        read_u32, read_u64, read_windows_datetime,
    },
};

#[derive(Debug, Error)]
pub enum PropertyStoreDataBlockParseError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid property store Version: expected 0x53505331, got 0x{0:08X}")]
    InvalidVersion(u32),
    #[error("string read error: {0}")]
    StringRead(#[from] StringReadError),
    #[error("bad TypedPropertyValue padding (must be 0)")]
    BadTpPadding,
    #[error("WindowsDateTime error: {0}")]
    WindowsDateTime(#[from] WindowsDateTimeError),
    #[error("wrong property type")]
    WrongPropertyType,
}

/// Raw typed value payload (verbatim [MS-OLEPS] TypedPropertyValue bytes).
#[derive(Debug, Clone)]
pub enum PropValue {
    Unparsed(u16, Vec<u8>),
    Unicode(String),
    WindowsDateTime(NaiveDateTime),
    U64(u64),
    Bool(bool),
}

/// One Serialized Property Storage (the only thing LNK embeds for this block).
#[derive(Debug, Clone)]
pub struct PropertyStore {
    pub unparsed_id_values: BTreeMap<u32, PropValue>,
    pub unparsed_name_values: BTreeMap<String, PropValue>,
    pub item_type_text: Option<String>,
    pub app_user_model_id: Option<String>,
    pub item_name_display: Option<String>,
    pub dual_mode: Option<bool>,
    pub size: Option<u64>,
    pub date_modified: Option<NaiveDateTime>,
    pub date_created: Option<NaiveDateTime>,
}

impl Default for PropertyStore {
    fn default() -> Self {
        Self {
            unparsed_id_values: BTreeMap::new(),
            unparsed_name_values: BTreeMap::new(),
            item_type_text: None,
            app_user_model_id: None,
            item_name_display: None,
            dual_mode: None,
            size: None,
            date_modified: None,
            date_created: None,
        }
    }
}

impl PropertyStore {
    pub fn parse(&mut self, r: &mut impl Read) -> Result<(), PropertyStoreDataBlockParseError> {
        // Serialized Property Storage header ([MS-PROPSTORE] as used by [MS-SHLLINK])
        let _storage_size = read_u32(r)?;
        let version = read_u32(r)?;
        if version != 0x5350_5331 {
            return Err(PropertyStoreDataBlockParseError::InvalidVersion(version));
        }
        let format_id = read_guid(r)?;
        let format_id_string = format_id.to_string();

        // Names are UTF-16 strings only for this special Format ID (FMTID_Storage)
        const FMTID_STORAGE: Guid = Guid {
            data1: 0xD5CDD505,
            data2: 0x2E9C,
            data3: 0x101B,
            data4: [0x93, 0x97, 0x08, 0x00, 0x2B, 0x2C, 0xF9, 0xAE],
        };

        loop {
            // Serialized Property Value — ends with ValueSize == 0
            let value_size = read_u32(r)?;
            if value_size == 0 {
                break;
            }

            if format_id == FMTID_STORAGE {
                // NameSize (u32), Reserved (u8), Name (UTF-16 bytes incl. NUL), Value (typed)
                let name_size = read_u32(r)? as usize;
                let _reserved = read_u8(r)?;
                let mut name_bytes = vec![0u8; name_size];
                r.read_exact(&mut name_bytes)?;
                // Use helper to decode the NUL-terminated UTF-16 from the slice
                let mut cur = Cursor::new(&name_bytes[..]);
                let name = read_c_utf16(&mut cur)?; // consumes until NUL, no need to manually trim :contentReference[oaicite:3]{index=3}

                let header_len = 4 /*ValueSize*/ + 4 /*NameSize*/ + 1 /*Reserved*/ + name_size;
                let tv_len = value_size.saturating_sub(header_len as u32) as usize;
                let mut tv_bytes = vec![0u8; tv_len];
                r.read_exact(&mut tv_bytes)?;

                let value = parse_typed_property_value(tv_bytes)?;
                self.unparsed_name_values.insert(name, value);

            // Remainder is the TypedPropertyValue
            } else {
                let id = read_u32(r)?;
                let _reserved = read_u8(r)?;

                let header_len = 4 /*ValueSize*/ + 4 /*NameSize*/ + 1 /*Reserved*/;
                let tv_len = value_size.saturating_sub(header_len as u32) as usize;
                let mut tv_bytes = vec![0u8; tv_len];
                r.read_exact(&mut tv_bytes)?;
                let value = parse_typed_property_value(tv_bytes)?;

                match format_id_string.as_str() {
                    "9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3" => match id {
                        5 => match value {
                            PropValue::Unicode(text) => self.app_user_model_id = Some(text),
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        11 => match value {
                            PropValue::Bool(b) => {
                                self.dual_mode = Some(b);
                            }
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        _ => {
                            self.unparsed_id_values.insert(id, value);
                        }
                    },
                    "B725F130-47EF-101A-A5F1-02608C9EEBAC" => match id {
                        4 => match value {
                            PropValue::Unicode(text) => self.item_type_text = Some(text),
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        10 => match value {
                            PropValue::Unicode(text) => self.item_name_display = Some(text),
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        12 => match value {
                            PropValue::U64(size) => self.size = Some(size),
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        14 => match value {
                            PropValue::WindowsDateTime(datetime) => {
                                self.date_created = Some(datetime)
                            }
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        15 => match value {
                            PropValue::WindowsDateTime(datetime) => {
                                self.date_modified = Some(datetime)
                            }
                            _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                        },
                        _ => {
                            self.unparsed_id_values.insert(id, value);
                        }
                    },
                    _ => {
                        self.unparsed_id_values.insert(id, value);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Parse [MS-OLEPS] TypedPropertyValue into your PropValue.
/// Unknown types are returned as Unparsed(raw_value_bytes).
fn parse_typed_property_value(buf: Vec<u8>) -> Result<PropValue, PropertyStoreDataBlockParseError> {
    let mut cur = Cursor::new(buf);

    let property_type = read_u16(&mut cur)?; // PropertyType
    let pad = read_u16(&mut cur)?; // MUST be zero
    if pad != 0 {
        return Err(PropertyStoreDataBlockParseError::BadTpPadding);
    }

    match property_type {
        0x000B => {
            // VT_BOOL -> Bool: 0x0000 = FALSE, 0xFFFF = TRUE
            let value = read_u16(&mut cur)?;
            let _padding = read_u16(&mut cur)?;
            Ok(PropValue::Bool(value != 0))
        }

        0x001F => {
            // VT_LPWSTR -> UnicodeString: Length (u32 chars incl. NUL), then UTF-16LE bytes, padded to 4
            let len_chars = read_u32(&mut cur)? as usize;
            let byte_len = len_chars.saturating_mul(2);
            let mut bytes = vec![0u8; byte_len];
            cur.read_exact(&mut bytes)?;

            // Consume padding to a 4-byte boundary inside the value
            let pad_len = (4 - (byte_len % 4)) % 4;
            if pad_len > 0 {
                let mut junk = [0u8; 3];
                cur.read_exact(&mut junk[..pad_len])?;
            }

            // Decode using your helper (NUL-terminated UTF-16)
            let mut name_cur = Cursor::new(&bytes[..]);
            let s = read_c_utf16(&mut name_cur)?; // stops at the first NUL :contentReference[oaicite:4]{index=4}
            Ok(PropValue::Unicode(s))
        }

        0x0040 => {
            // VT_FILETIME -> NaiveDateTime via helper
            let dt = read_windows_datetime(&mut cur)?; // FILETIME 100ns since 1601-01-01 → NaiveDateTime :contentReference[oaicite:5]{index=5}
            Ok(PropValue::WindowsDateTime(dt))
        }

        0x0015 => {
            // VT_UI8 -> U64
            let v = read_u64(&mut cur)?;
            Ok(PropValue::U64(v))
        }

        _ => {
            // Return the raw Value bytes (excluding the 4-byte Type/Pad header)
            Ok(PropValue::Unparsed(
                property_type,
                cur.into_inner().into_iter().skip(4).collect(),
            ))
        }
    }
}
