use clipboard::{ClipboardContext, ClipboardProvider};
use flume::Sender;
use reqwest::Client;
use util::{Device, DeviceUpdate, Login};

pub struct ExposedState {
    pub devices: Vec<Device>,
    pub login: Option<Login>,
}

impl ExposedState {
    pub fn new() -> Self {
        ExposedState {
            devices: vec![],
            login: None,
        }
    }
    pub fn copy_to_clipboard(&self) {
        ClipboardProvider::new()
            .ok()
            .and_then(|mut ctx: ClipboardContext| {
                ctx.set_contents(
                    self.devices
                        .iter()
                        .map(|d| format!("{},{},{}", d.cc, d.ui_type, d.description))
                        .fold(String::new(), |acc, d| format!("{}{}\n", acc, d))
                        .to_owned(),
                )
                .ok()
            });
    }

    pub async fn update_device(
        &mut self,
        device: DeviceUpdate,
        slint_device_tx: Sender<Vec<Device>>,
    ) {
        println!("login ? ");
        if let Some(login) = &self.login {
            let device_clone = device.clone();

            if let Ok(response) = Client::new()
                .post(format!(
                    "http://{}/devices?password={}",
                    login.url, login.pass
                ))
                .json(&device_clone)
                .send()
                .await
            {
                if let Ok(devices) = response.json::<Vec<Device>>().await {
                    println!("{devices:?}");
                    self.devices = devices.clone();
                    let _ = slint_device_tx.send(devices);
                };
            };
        };
    }
}
