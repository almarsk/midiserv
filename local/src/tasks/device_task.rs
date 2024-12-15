use std::sync::Arc;

use flume::{Receiver, Sender};
use tokio::sync::Mutex;
use util::{DeviceCmd, Login};

pub fn device_task(
    shutdown: Receiver<bool>,
    cmd: Receiver<DeviceCmd>,
    rpns: Sender<Vec<String>>,
    login: Login,
) {
    tokio::spawn(async move {
        let exposed_devices = util::ExposedDevices::new(login);
        let exposed = Arc::new(Mutex::new(exposed_devices));
        let exposed = Arc::clone(&exposed);
        loop {
            tokio::select! {
                shutdown_option = shutdown.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
                exposed_device_command = cmd.recv_async() => {
                    if let Ok(e) = exposed_device_command {
                        match e {
                            DeviceCmd::CopyToClipBoard => exposed.lock().await.copy_to_clipboard(),
                            DeviceCmd::GetJoined => {
                                let _ = rpns
                                    .send_async(exposed.lock().await.get_joined())
                                    .await;
                                },
                            DeviceCmd::Update(update) => {let _ = exposed.lock().await.update_device(update).await;}
                        }
                    }
                }
            }
        }
    });
}
