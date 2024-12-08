#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod exposed_devices;
mod midi;

use anyhow::Result;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use exposed_devices::Device;
use exposed_devices::DeviceCommand;
use flume::bounded;
use flume::Receiver;
use flume::Sender;
use midi::MidiCommand;
use slint::CloseRequestResponse;
use slint::ComponentHandle;
use slint::{ModelRc, SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let app = AppWindow::new()?;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let (shutdown_tx, shutdown_rx): (Sender<bool>, Receiver<bool>) = bounded(10);

    let mut midi = midi::Midi::new();

    let p = Rc::new(
        midi.get_ports()
            .iter()
            .map(|s| SharedString::from(s))
            .collect::<VecModel<_>>(),
    );
    app.global::<AppState>()
        .set_midi_ports(ModelRc::from(Rc::clone(&p)));

    let exp_dev = Rc::new(VecModel::from(vec![]));
    app.set_exposed_devices(ModelRc::from(Rc::clone(&exp_dev)));

    let (device_command_tx, device_command_rx): (Sender<DeviceCommand>, Receiver<DeviceCommand>) =
        bounded(10);

    let (device_response_tx, device_response_rx): (Sender<Vec<String>>, Receiver<Vec<String>>) =
        bounded(10);

    let shutdown_rx_clone = shutdown_rx.clone();
    rt.spawn(async move {
        let exposed_devices = exposed_devices::ExposedDevices::new();
        let exposed = Arc::new(Mutex::new(exposed_devices));
        let exposed = Arc::clone(&exposed);
        loop {
            tokio::select! {
                exposed_device_command = device_command_rx.recv_async() => {
                    if let Ok(e) = exposed_device_command {
                        match e {
                            DeviceCommand::Push(d) => exposed.lock().await.push(d),
                            DeviceCommand::Remove(index) => exposed.lock().await.remove(index),
                            DeviceCommand::Clear => exposed.lock().await.clear(),
                            DeviceCommand::CopyToClipBoard => {
                                exposed.lock().await.copy_to_clipboard()
                            },
                            DeviceCommand::GetJoined => {
                                let _ = device_response_tx
                                    .send_async(exposed.lock().await.get_joined())
                                    .await;
                                }
                        }
                    }
                }
                shutdown_option = shutdown_rx_clone.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
            }
        }
    });

    let (midi_tx, midi_rx): (Sender<MidiCommand>, Receiver<MidiCommand>) = bounded(10);

    let shutdown_rx_clone = shutdown_rx.clone();
    rt.spawn(async move {
        let midi = Arc::new(Mutex::new(midi));
        let midi = Arc::clone(&midi);
        loop {
            tokio::select! {
                command_option = midi_rx.recv_async() => {
                 if let Ok(command) = command_option {
                     let mut midi = midi.lock().await;
                     match command {
                         MidiCommand::Dummy(cc) => {
                             midi.send_cc(cc, 0)
                         }
                         MidiCommand::Port(port) => midi.update_port(port),
                     }
                    }
                }
                shutdown_option = shutdown_rx_clone.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
            }
        }
    });

    let tx_clone = midi_tx.clone();
    app.global::<AppState>().on_choose_midi_port(move |port| {
        let _ = tx_clone.send(MidiCommand::Port(port as usize));
    });

    let tx_clone = midi_tx.clone();
    app.global::<AppState>()
        .on_send_dummy_cc(move |controller| {
            controller
                .clone()
                .parse::<u8>()
                .ok()
                .and_then(|cc| tx_clone.send(MidiCommand::Dummy(cc)).ok());
        });

    let state = Rc::new(ExposedState {
        tx: device_command_tx.clone(),
        rx: device_response_rx.clone(),
        exp_dev,
    });

    let state_clone = state.clone();
    app.global::<AppState>()
        .on_expose_device(move |cc, ui_type, description| {
            if let Some(new_device) = Device::from_string_args(
                cc.to_string(),
                ui_type.to_string(),
                description.to_string(),
            ) {
                let state_clone = state_clone.clone();
                let _ = slint::spawn_local(async move {
                    let _ = state_clone.tx.send(DeviceCommand::Push(new_device));

                    update_exp_dev(state_clone.to_owned());
                });
            }
        });

    let state_clone = state.clone();
    app.global::<AppState>().on_hide_device(move |i| {
        if let Ok(index) = i.parse::<usize>() {
            let _ = state_clone.tx.send(DeviceCommand::Remove(index));
            update_exp_dev(state_clone.to_owned());
        }
    });

    let state_clone = state.clone();
    app.global::<AppState>().on_copy_to_clipboard(move || {
        let _ = state_clone.tx.send(DeviceCommand::CopyToClipBoard);
    });

    let state_clone = state.clone();
    app.global::<AppState>().on_paste(move || {
        if let Ok(ctx) = ClipboardProvider::new() {
            let mut ctx: ClipboardContext = ctx;
            if let Ok(content) = ctx.get_contents() {
                let mut rdr = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_reader(content.as_bytes());

                for result in rdr.records() {
                    if let Ok(record) = result {
                        if let (Some(cc), Some(ui_type), Some(desc)) =
                            (record.get(0), record.get(1), record.get(2))
                        {
                            if let Some(new_device) = Device::from_string_args(
                                cc.to_string(),
                                ui_type.to_string(),
                                desc.to_string(),
                            ) {
                                let _ = state_clone.tx.send(DeviceCommand::Push(new_device));
                            }
                        }
                    }
                }
                update_exp_dev(state_clone.clone());
            }
        }
    });

    let state_clone = state.clone();
    app.global::<AppState>().on_clear_all(move || {
        let _ = state_clone.tx.send(DeviceCommand::Clear);
        update_exp_dev(state_clone.to_owned());
    });

    let app_clone = app.clone_strong();
    app.global::<AppState>().on_login(move |_url, _pass| {
        app_clone.global::<AppState>().set_connected_to_server(true);
    });

    let app_clone = app.clone_strong();
    app.global::<AppState>().on_disconnect(move || {
        app_clone
            .global::<AppState>()
            .set_connected_to_server(false);
    });

    app.window().on_close_requested(move || {
        let _ = shutdown_tx.send(true);
        CloseRequestResponse::HideWindow
    });
    let _ = app.run();

    rt.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    });
    Ok(())
}

struct ExposedState {
    tx: Sender<DeviceCommand>,
    rx: Receiver<Vec<String>>,
    exp_dev: Rc<VecModel<SharedString>>,
}

fn update_exp_dev(state: Rc<ExposedState>) {
    let _ = slint::spawn_local(async move {
        let _ = state.tx.send(DeviceCommand::GetJoined);
        state.exp_dev.set_vec(
            state
                .rx
                .recv_async()
                .await
                .map(|v| v.iter().map(|s| SharedString::from(s)).collect())
                .unwrap_or(vec![]),
        )
    });
}
