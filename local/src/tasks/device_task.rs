use std::sync::Arc;

use flume::{Receiver, Sender};
use tokio::{runtime::Runtime, sync::Mutex};
use util::DeviceCmd;

pub fn device_task(
    rt: &Runtime,
    shutdown: Receiver<bool>,
    cmd: Receiver<DeviceCmd>,
    rpns: Sender<Vec<String>>,
) {
    rt.spawn(async move {
        let exposed_devices = util::ExposedDevices::new();
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
                            DeviceCmd::Push(d) => exposed.lock().await.push(d),
                            DeviceCmd::Remove(index) => exposed.lock().await.remove(index),
                            DeviceCmd::Clear => exposed.lock().await.clear(),
                            DeviceCmd::CopyToClipBoard => exposed.lock().await.copy_to_clipboard(),
                            DeviceCmd::GetJoined => {
                                let _ = rpns
                                    .send_async(exposed.lock().await.get_joined())
                                    .await;
                                }
                        }
                    }
                }
            }
        }
    });
}
