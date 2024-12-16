use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use strum::{EnumIter, IntoEnumIterator};

#[derive(EnumIter, Clone, Serialize, Deserialize, Debug)]
pub enum UIType {
    Slide,
    Check,
}

impl fmt::Display for UIType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UIType::Slide => write!(f, "slide"),
            UIType::Check => write!(f, "check"),
        }
    }
}

impl FromStr for UIType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "slide" => Ok(UIType::Slide),
            "check" => Ok(UIType::Check),
            _ => Err(format!("'{}' is not a valid value for UIType", s)),
        }
    }
}

impl UIType {
    pub fn to_vec() -> Vec<String> {
        UIType::iter().fold(Vec::new(), |mut acc, variant| {
            acc.push(variant.to_string());
            acc
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Device {
    pub cc: u8,
    pub ui_type: UIType,
    pub description: String,
}

impl Device {
    pub fn new(cc: u8, ui_type: UIType, description: String) -> Self {
        Device {
            cc,
            ui_type,
            description,
        }
    }

    pub fn from_string_args(cc: String, ui_type: String, description: String) -> Option<Self> {
        cc.parse::<u8>().ok().and_then(|controller| {
            UIType::from_str(ui_type.as_str())
                .ok()
                .map(|ui_type| Device::new(controller, ui_type, description))
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DeviceUpdate {
    Add(Vec<Device>),
    Remove(Vec<usize>),
    Clear,
}
