use flume::{Receiver, Sender};
use tokio::runtime::Runtime;
use util::{Device, DeviceCmd, DeviceUpdate};

use crate::exposed_state::ExposedState;

pub fn device_task(
    rt: &Runtime,
    shutdown: Receiver<bool>,
    command: Receiver<DeviceCmd>,
    mut state: ExposedState,
    slint_device_tx: Sender<Vec<Device>>,
) {
    rt.spawn(async move {
        loop {
            tokio::select! {
                shutdown_option = shutdown.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
                exposed_device_command = command.recv_async() => {
                    if let Ok(e) = exposed_device_command {

                        println!("{:?}", e);

                        match e {
                            DeviceCmd::Login(login) => {state.login = Some(login);},
                            DeviceCmd::CopyToClipboard => {let _ = &state.copy_to_clipboard();},
                            DeviceCmd::Update(update) => {

                                let update = if let DeviceUpdate::Remove(indexes) = update {
                                    DeviceUpdate::Remove(
                                        indexes.into_iter()
                                            .filter_map(|i| state.devices.get(i))
                                            .map(|d| d.cc as usize).collect::<Vec<_>>())
                                } else {
                                    update
                                };

                                println!("{:?}", update);

                                let _ = &state.update_device(update, slint_device_tx.clone()).await;
                            }
                        }
                    }
                }
            }
        }
    });
}
