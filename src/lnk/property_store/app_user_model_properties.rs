use crate::lnk::property_store::{PropValue, PropertyStoreDataBlockParseError};

#[derive(Debug, Clone, Default)]
pub struct AppUserModelProperties {
    /// PID 5 – System.AppUserModel.ID
    pub id: Option<String>,

    /// PID 6 – System.AppUserModel.RelaunchCommand
    pub relaunch_command: Option<String>,

    /// PID 7 – System.AppUserModel.RelaunchDisplayNameResource
    pub relaunch_display_name_resource: Option<String>,

    /// PID 8 – System.AppUserModel.RelaunchIconResource
    pub relaunch_icon_resource: Option<String>,

    /// PID 11 – System.AppUserModel.IsDualMode
    pub is_dual_mode: Option<bool>,
}

impl AppUserModelProperties {
    pub(crate) fn from_raw(
        properties: Vec<(u32, PropValue)>,
    ) -> Result<Self, PropertyStoreDataBlockParseError> {
        let mut me = Self::default();

        for (pid, value) in properties {
            println!("PID: {pid}");
            println!("Value: {value:?}");
            match pid {
                // PID 5 – System.AppUserModel.ID (VT_LPWSTR)
                5 => match value {
                    PropValue::Unicode(text) => me.id = Some(text),
                    _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                },

                // PID 6 – System.AppUserModel.RelaunchCommand (VT_LPWSTR)
                6 => match value {
                    PropValue::Unicode(text) => me.relaunch_command = Some(text),
                    _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                },

                // PID 7 – System.AppUserModel.RelaunchDisplayNameResource (VT_LPWSTR)
                7 => match value {
                    PropValue::Unicode(text) => me.relaunch_display_name_resource = Some(text),
                    _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                },

                // PID 8 – System.AppUserModel.RelaunchIconResource (VT_LPWSTR)
                8 => match value {
                    PropValue::Unicode(text) => me.relaunch_icon_resource = Some(text),
                    _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                },

                // PID 11 – System.AppUserModel.IsDualMode (VT_BOOL)
                11 => match value {
                    PropValue::Bool(b) => me.is_dual_mode = Some(b),
                    _ => return Err(PropertyStoreDataBlockParseError::WrongPropertyType),
                },

                _ => return Err(PropertyStoreDataBlockParseError::UnknownPropertyId(pid)),
            }
        }

        Ok(me)
    }
}
