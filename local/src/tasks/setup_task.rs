use std::sync::Arc;

use flume::{Receiver, Sender};
use futures_util::stream::StreamExt;
use tokio::{runtime::Runtime, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use util::{DeviceCmd, MidiCmd};

use crate::{Login, Status};

use super::device_task;

pub fn login_task(
    rt: &Runtime,
    shutdown_rx: Receiver<bool>,
    login_rx: Receiver<Login>,
    midi_tx: Sender<MidiCmd>,
    login_tx: Sender<bool>,
    login_data: Arc<Mutex<Option<Login>>>,
    status_tx: Sender<Status>,
    passthrough: Arc<Mutex<bool>>,
    logout: Receiver<()>,
    dvc_rx: Receiver<DeviceCmd>,
    dvc_rpns_tx: Sender<Vec<String>>,
) {
    rt.spawn(async move {
        loop {
            tokio::select! {
                shutdown_option = shutdown_rx.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
                login = login_rx.recv_async() => {
                    if let Ok(login) = login {
                        {
                            let mut guard = login_data.lock().await;
                            *guard = Some(login.clone());
                        };
                        if let Err(_) = setup_connection(
                            login,
                            login_tx.clone(),
                            midi_tx.clone(),
                            status_tx.clone(),
                            passthrough.clone(),
                            logout.clone(),
                            shutdown_rx.clone(),
                            dvc_rx.clone(),
                            dvc_rpns_tx.clone()
                        ).await {
                            let _ = login_tx.send(false);
                        };
                    }
                }
            }
        }
    });
}

async fn setup_connection(
    login: Login,
    login_tx: Sender<bool>,
    midi_tx: Sender<MidiCmd>,
    status_tx: Sender<Status>,
    passthrough: Arc<Mutex<bool>>,
    logout: Receiver<()>,
    shutdown: Receiver<bool>,
    dvc_rx: Receiver<DeviceCmd>,
    dvc_rpns_tx: Sender<Vec<String>>,
) -> Result<(), ()> {
    let ws_url = format!("ws://{}/login?password={}", login.url, login.pass);

    match connect_async(ws_url).await {
        Ok((mut ws_stream, _)) => {
            let _ = login_tx.send(true);
            let _ = status_tx.send(Status::Connection(true));

            device_task(shutdown.clone(), dvc_rx, dvc_rpns_tx, login.clone());

            loop {
                tokio::select! {
                    message = ws_stream.next() => {
                        if let Some(message) = message {
                            match message {
                                Ok(message) => match message {
                                    Message::Text(text) => {
                                        let _ = status_tx.send_async(Status::Text(text)).await;
                                    }
                                    _ => {
                                        if *passthrough.lock().await {
                                            if let Some((cc, value)) = parse_message(message.into_data()) {
                                                let _ = midi_tx.send_async(MidiCmd::Signal(cc, value)).await;
                                            };
                                        }
                                    }
                                },
                                Err(_) => {
                                    let _ = status_tx.send_async(Status::Connection(false)).await;
                                }
                            }
                        }
                    }

                    _ = logout.recv_async() => {
                        let _ = ws_stream.close(None).await;
                        break;
                    }
                }
            }
        }
        Err(e) => {
            let _ = login_tx.send(false);
            eprintln!("{e}");
        }
    };
    Ok(())
}

fn parse_message(data: Vec<u8>) -> Option<(u8, u8)> {
    let cc = data.get(0)?;
    let value = data.get(1)?;
    Some((*cc, *value))
}
