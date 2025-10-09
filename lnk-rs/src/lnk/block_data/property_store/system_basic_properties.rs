use chrono::NaiveDateTime;

use super::{PropValue, PropertyStoreDataBlockParseError};

#[derive(Debug, Clone, Default)]
pub struct SystemBasicProperties {
    /// PID 4 – System.ItemTypeText (user-friendly type name)
    pub item_type_text: Option<String>,

    /// PID 10 – System.ItemNameDisplay (file name)
    pub item_name_display: Option<String>,

    /// PID 12 – System.Size (bytes)
    pub size: Option<u64>,

    /// PID 14 – System.DateModified
    pub date_modified: Option<NaiveDateTime>,

    /// PID 15 – System.DateCreated
    pub date_created: Option<NaiveDateTime>,
}
impl SystemBasicProperties {
    pub(crate) fn from_raw(
        properties: Vec<(u32, PropValue)>,
    ) -> Result<Self, PropertyStoreDataBlockParseError> {
        let mut me = Self::default();

        for (pid, value) in properties {
            match pid {
                // PID 4 – System.ItemTypeText (VT_LPWSTR)
                4 => match value {
                    PropValue::Unicode(s) => me.item_type_text = Some(s),
                    _ => {
                        return Err(PropertyStoreDataBlockParseError::WrongPropertyType);
                    }
                },

                // PID 10 – System.ItemNameDisplay (VT_LPWSTR)
                10 => match value {
                    PropValue::Unicode(s) => me.item_name_display = Some(s),
                    _ => {
                        return Err(PropertyStoreDataBlockParseError::WrongPropertyType);
                    }
                },

                // PID 12 – System.Size (VT_UI8)
                12 => match value {
                    PropValue::U64(n) => me.size = Some(n),
                    _ => {
                        return Err(PropertyStoreDataBlockParseError::WrongPropertyType);
                    }
                },

                // PID 14 – System.DateModified (VT_FILETIME)
                14 => match value {
                    PropValue::WindowsDateTime(dt) => me.date_modified = Some(dt),
                    _ => {
                        return Err(PropertyStoreDataBlockParseError::WrongPropertyType);
                    }
                },

                // PID 15 – System.DateCreated (VT_FILETIME)
                15 => match value {
                    PropValue::WindowsDateTime(dt) => me.date_created = Some(dt),
                    _ => {
                        return Err(PropertyStoreDataBlockParseError::WrongPropertyType);
                    }
                },

                _ => return Err(PropertyStoreDataBlockParseError::UnknownPropertyId(pid)),
            }
        }

        Ok(me)
    }
}
