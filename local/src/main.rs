#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod setters;
mod tasks;
mod ui_handlers;

use anyhow::Result;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use flume::bounded;
use flume::Receiver;
use flume::Sender;
use reqwest::Client;
use setters::init_exposed_devices;
use setters::init_ui_types;
use setters::set_ports;
use slint::CloseRequestResponse;
use slint::ComponentHandle;
use slint::{SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use tasks::login_task;
use tokio::sync::Mutex;
use util::Device;
use util::DeviceCmd;
use util::MidiCmd;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // init
    let app = AppWindow::new()?;
    let mut midi = util::Midi::new();

    set_ports(&mut midi, app.clone_strong());
    let exp_dev = init_exposed_devices(app.clone_strong());
    init_ui_types(app.clone_strong());

    let login: Arc<Mutex<Option<Login>>> = Arc::new(Mutex::new(None));
    let rt = tokio::runtime::Runtime::new().unwrap();

    // channels
    let (shutdown_tx, shutdown_rx): (Sender<bool>, Receiver<bool>) = bounded(10);
    let (midi_tx, midi_rx): (Sender<MidiCmd>, Receiver<MidiCmd>) = bounded(10);
    let (dvc_tx, dvc_rx): (Sender<DeviceCmd>, Receiver<DeviceCmd>) = bounded(10);
    let (dvc_rpns_tx, dvc_rpns_rx): (Sender<Vec<String>>, Receiver<Vec<String>>) = bounded(10);
    let (login_tx, login_rx): (Sender<Login>, Receiver<Login>) = bounded(10);
    let (logout_tx, logout_rx): (Sender<()>, Receiver<()>) = bounded(10);
    let (login_response_tx, login_response_rx): (Sender<bool>, Receiver<bool>) = bounded(10);

    // exposed devices state
    let state = Rc::new(ExposedState {
        tx: dvc_tx.clone(),
        rx: dvc_rpns_rx.clone(),
        exp_dev,
    });

    let shutdown_rx_clone = shutdown_rx.clone();
    let logout_rx_clone = logout_rx.clone();
    let login_clone = login.clone();
    rt.spawn(async move {
        loop {
            tokio::select! {
                shutdown_option = shutdown_rx_clone.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
                _ = logout_rx_clone.recv_async() => {
                    if let Some(ref login) = *login_clone.lock().await {
                    let url = format!("http://{}/logout", &login.url);
                    let r = Client::new().get(url).send().await;
                    println!("disconnect sent {:?}", r);
                    }
                }
            }
        }
    });

    login_task(
        &rt,
        shutdown_rx.clone(),
        login_rx,
        midi_tx.clone(),
        login_response_tx.clone(),
        login.clone(),
    );

    let shutdown_rx_clone = shutdown_rx.clone();
    rt.spawn(async move {
        let exposed_devices = util::ExposedDevices::new();
        let exposed = Arc::new(Mutex::new(exposed_devices));
        let exposed = Arc::clone(&exposed);
        loop {
            tokio::select! {
                exposed_device_command = dvc_rx.recv_async() => {
                    if let Ok(e) = exposed_device_command {
                        match e {
                            DeviceCmd::Push(d) => exposed.lock().await.push(d),
                            DeviceCmd::Remove(index) => exposed.lock().await.remove(index),
                            DeviceCmd::Clear => exposed.lock().await.clear(),
                            DeviceCmd::CopyToClipBoard => exposed.lock().await.copy_to_clipboard(),
                            DeviceCmd::GetJoined => {
                                let _ = dvc_rpns_tx
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
                         MidiCmd::Dummy(cc) => midi.send_cc(cc, 0),
                         MidiCmd::Signal(cc, value) => midi.send_cc(cc, value),
                         MidiCmd::Port(port) => midi.update_port(port),
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
        let _ = tx_clone.send(MidiCmd::Port(port as usize));
    });

    let tx_clone = midi_tx.clone();
    app.global::<AppState>()
        .on_send_dummy_cc(move |controller| {
            controller
                .clone()
                .parse::<u8>()
                .ok()
                .and_then(|cc| tx_clone.send(MidiCmd::Dummy(cc)).ok());
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
                    let _ = state_clone.tx.send(DeviceCmd::Push(new_device));

                    update_exp_dev(state_clone.to_owned());
                });
            }
        });

    let state_clone = state.clone();
    app.global::<AppState>().on_hide_device(move |i| {
        if let Ok(index) = i.parse::<usize>() {
            let _ = state_clone.tx.send(DeviceCmd::Remove(index));
            update_exp_dev(state_clone.to_owned());
        }
    });

    let state_clone = state.clone();
    app.global::<AppState>().on_copy_to_clipboard(move || {
        let _ = state_clone.tx.send(DeviceCmd::CopyToClipBoard);
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
                                let _ = state_clone.tx.send(DeviceCmd::Push(new_device));
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
        let _ = state_clone.tx.send(DeviceCmd::Clear);
        update_exp_dev(state_clone.to_owned());
    });

    let app_clone = app.clone_strong();
    app.global::<AppState>().on_login(move |url, pass| {
        let _ = login_tx.send(Login {
            url: url.to_string(),
            pass: pass.to_string(),
        });

        app_clone
            .global::<AppState>()
            .set_connected_to_server(login_response_rx.recv().unwrap_or(false));
    });

    let app_clone = app.clone_strong();
    let logout_tx_clone = logout_tx.clone();
    app.global::<AppState>().on_disconnect(move || {
        let r = logout_tx_clone.send(());
        println!("ds {:?}", r);
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

#[derive(Clone)]
struct Login {
    pub url: String,
    pub pass: String,
}

struct ExposedState {
    tx: Sender<DeviceCmd>,
    rx: Receiver<Vec<String>>,
    exp_dev: Rc<VecModel<SharedString>>,
}

fn update_exp_dev(state: Rc<ExposedState>) {
    let _ = slint::spawn_local(async move {
        let _ = state.tx.send(DeviceCmd::GetJoined);
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
