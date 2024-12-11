use clipboard::{ClipboardContext, ClipboardProvider};
use std::{fmt, str::FromStr};
use strum::{EnumIter, IntoEnumIterator};

pub enum DeviceCommand {
    Push(Device),
    Remove(usize),
    Clear,
    CopyToClipBoard,
    GetJoined,
}

#[derive(EnumIter, Clone)]
pub enum UIType {
    Empty,
    Check,
}

impl fmt::Display for UIType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UIType::Empty => write!(f, "{}", "empty"),
            UIType::Check => write!(f, "{}", "check"),
        }
    }
}

impl FromStr for UIType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "empty" => Ok(UIType::Empty),
            _ => Err(format!("'{}' is not a valid value for MyEnum", s)),
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

#[derive(Clone)]
pub struct Device {
    cc: u8,
    ui_type: UIType,
    description: String,
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
                .and_then(|ui_type| Some(Device::new(controller, ui_type, description)))
        })
    }
}

pub struct ExposedDevices {
    joined: Vec<String>,
    devices: Vec<Device>,
}

impl ExposedDevices {
    pub fn new() -> Self {
        ExposedDevices {
            joined: vec![],
            devices: vec![],
        }
    }

    fn update_joined(&mut self) {
        self.joined = self
            .devices
            .iter()
            .map(|d| format!("{}|{}|{}", d.cc, d.ui_type, d.description))
            .collect::<Vec<String>>()
    }

    pub fn get_joined(&self) -> Vec<String> {
        self.joined.clone()
    }

    pub fn get_joined_string(&self) -> String {
        self.devices
            .iter()
            .map(|d| d.clone())
            .fold(String::new(), |acc, d| {
                format!("{}{},{},{}\n", acc, d.cc, d.ui_type, d.description)
            })
    }

    pub fn clear(&mut self) {
        self.joined = vec![];
        self.devices = vec![];
    }

    pub fn push(&mut self, device: Device) {
        self.devices.push(device);
        self.update_joined();
    }

    pub fn remove(&mut self, index: usize) {
        if let Some(_) = self.devices.get(index) {
            self.devices.remove(index);
            self.update_joined();
        }
    }

    pub fn copy_to_clipboard(&self) {
        ClipboardProvider::new()
            .ok()
            .and_then(|mut ctx: ClipboardContext| {
                ctx.set_contents(self.get_joined_string().to_owned()).ok()
            });
    }
}
