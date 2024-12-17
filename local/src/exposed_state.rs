use flume::Sender;
use reqwest::Client;
use util::{copy_to_clipboard, Device, DeviceUpdate, Login};

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
        copy_to_clipboard(
            self.devices
                .iter()
                .map(|d| format!("{},{},{}", d.cc, d.ui_type, d.description))
                .fold(String::new(), |acc, d| format!("{}{}\n", acc, d))
                .to_owned(),
        );
    }

    pub async fn paste(&mut self, slint_device_tx: Sender<Vec<Device>>, content: String) {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(content.as_bytes());

        let new_devices = rdr
            .records()
            .into_iter()
            .filter_map(|r| {
                if let Ok(record) = r {
                    if let (Some(cc), Some(ui_type), Some(desc)) =
                        (record.get(0), record.get(1), record.get(2))
                    {
                        Device::from_string_args(
                            cc.to_string(),
                            ui_type.to_string(),
                            desc.to_string(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<Device>>();

        self.update_device(DeviceUpdate::Add(new_devices), slint_device_tx)
            .await;
    }

    pub async fn update_device(
        &mut self,
        device: DeviceUpdate,
        slint_device_tx: Sender<Vec<Device>>,
    ) {
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
                    //println!("{devices:?}");
                    self.devices = devices.clone();
                    let _ = slint_device_tx.send(devices);
                };
            };
        };
    }
}
