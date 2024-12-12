use std::sync::Arc;

use flume::{Receiver, Sender};
use futures_util::stream::StreamExt;
use tokio::{runtime::Runtime, sync::Mutex};
use tokio_tungstenite::connect_async;
use util::MidiCmd;

use crate::Login;

pub fn login_task(
    rt: &Runtime,
    shut_rx: Receiver<bool>,
    login_rx: Receiver<Login>,
    midi_tx: Sender<MidiCmd>,
    login_tx: Sender<bool>,
    login_data: Arc<Mutex<Option<Login>>>,
) {
    rt.spawn(async move {
        loop {
            tokio::select! {
                shutdown_option = shut_rx.recv_async() => {
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
                        if let Err(_) = process_response(
                            login,
                            login_tx.clone(),
                            midi_tx.clone(),
                        ).await {
                            let _ = login_tx.send(false);
                        };
                    }
                }
            }
        }
    });
}

async fn process_response(
    login: Login,
    login_tx: Sender<bool>,
    midi_tx: Sender<MidiCmd>,
) -> Result<(), ()> {
    let ws_url = format!("ws://{}/login?password={}", login.url, login.pass);
    match connect_async(ws_url).await {
        Ok((mut ws_stream, _)) => {
            let _ = login_tx.send(true);
            while let Some(Ok(message)) = ws_stream.next().await {
                if let Some((cc, value)) = parse_message(message.into_data()) {
                    let _ = midi_tx.send(MidiCmd::Signal(cc, value));
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
