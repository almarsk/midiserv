use clipboard::{ClipboardContext, ClipboardProvider};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use strum::{EnumIter, IntoEnumIterator};

use crate::Login;

pub enum DeviceCmd {
    Update(DeviceUpdate),
    CopyToClipBoard,
    GetJoined,
}

#[derive(EnumIter, Clone, Serialize, Deserialize, Debug)]
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Device {
    pub cc: u8,
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

    login: Login,
}

impl ExposedDevices {
    pub fn new(login: Login) -> Self {
        ExposedDevices {
            joined: vec![],

            login,
        }
    }

    pub fn get_joined(&self) -> Vec<String> {
        println!("{:?}", self.joined);
        self.joined.clone()
    }

    pub fn get_joined_string(&self) -> String {
        self.joined
            .iter()
            .map(|d| d.clone())
            .fold(String::new(), |acc, d| format!("{}\n{}", acc, d))
    }

    pub fn clear(&mut self) {
        self.joined = vec![];
    }

    pub async fn update_device(&mut self, device: DeviceUpdate) -> Result<(), reqwest::Error> {
        // send request
        let response = Client::new()
            .post(format!(
                "http://{}/devices?password={}",
                self.login.url, self.login.pass
            ))
            .json(&device)
            .send()
            .await?;

        if let Ok(devices) = response.json::<Vec<Device>>().await {
            println!("{devices:?}");
            self.joined = devices
                .iter()
                .map(|d| format!("{}|{}|{}", d.cc, d.ui_type, d.description))
                .collect::<Vec<String>>();
        } else {
            println!("problema");
        }

        Ok(())
    }

    pub fn copy_to_clipboard(&self) {
        ClipboardProvider::new()
            .ok()
            .and_then(|mut ctx: ClipboardContext| {
                ctx.set_contents(self.get_joined_string().to_owned()).ok()
            });
    }
}

#[derive(Serialize, Deserialize)]
pub enum DeviceUpdate {
    Add(Device),
    Remove(u8),
    Clear,
}
