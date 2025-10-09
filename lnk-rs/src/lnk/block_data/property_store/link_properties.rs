use super::{PropValue, PropertyStoreDataBlockParseError};
use crate::lnk::helpers::Guid;

#[derive(Debug, Default)]
pub struct LinkProperties {
    volume_id: Option<Guid>,
}

impl LinkProperties {
    pub(crate) fn from_raw(
        properties: Vec<(u32, PropValue)>,
    ) -> Result<Self, PropertyStoreDataBlockParseError> {
        let mut me = Self::default();

        for (pid, value) in properties {
            match pid {
                // PID 104 – System.Link.VolumeId
                104 => match value {
                    PropValue::Guid(s) => me.volume_id = Some(s),
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
