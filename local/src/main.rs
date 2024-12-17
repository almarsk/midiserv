#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod exposed_state;
mod setters;
mod tasks;
mod ui_handlers;

use anyhow::Result;
use exposed_state::ExposedState;
use flume::bounded;
use flume::Receiver;
use flume::Sender;
use setters::{connection_status, init_ui_types, set_ports, Status};
use slint::CloseRequestResponse;
use slint::ComponentHandle;
use slint::ModelRc;
use slint::SharedString;
use slint::VecModel;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tasks::device_task;
use tasks::midi_task;
use tasks::setup_task;
use tokio::sync::Mutex;
use util::Device;
use util::DeviceCmd;
use util::DeviceUpdate;
use util::Login;
use util::Midi;
use util::MidiCmd;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // INIT
    let app = AppWindow::new()?;
    let midi = Arc::new(Mutex::new(Midi::new()));
    init_ui_types(app.clone_strong());
    let state = ExposedState::new();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let passthrough = Arc::new(Mutex::new(true));

    let passthrough_clone = passthrough.clone();
    let app_clone = app.clone_strong();
    let _ = slint::spawn_local(async move {
        app_clone
            .global::<AppState>()
            .set_passthrough(*passthrough_clone.lock().await);
    });

    // CHANNELS
    let (shutdown_tx, shutdown_rx): (Sender<bool>, Receiver<bool>) = bounded(10);
    let (midi_tx, midi_rx): (Sender<MidiCmd>, Receiver<MidiCmd>) = bounded(10);
    let (login_tx, login_rx): (Sender<Login>, Receiver<Login>) = bounded(10);
    let (logout_tx, logout_rx): (Sender<()>, Receiver<()>) = bounded(10);
    let (login_response_tx, login_response_rx): (Sender<bool>, Receiver<bool>) = bounded(10);
    let (status_tx, status_rx): (Sender<Status>, Receiver<Status>) = bounded(10);
    let (device_tx, device_rx): (Sender<DeviceCmd>, Receiver<DeviceCmd>) = bounded(10);
    let (slint_device_tx, slint_device_rx): (Sender<Vec<Device>>, Receiver<Vec<Device>>) =
        bounded(1);

    // TASKS
    let app_clone = app.clone_strong();
    connection_status(app_clone, status_rx, device_tx.clone());
    setup_task(
        &rt,
        shutdown_rx.clone(),
        login_rx,
        midi_tx.clone(),
        login_response_tx.clone(),
        status_tx.clone(),
        passthrough.clone(),
        logout_rx.clone(),
    );
    device_task(&rt, shutdown_rx.clone(), device_rx, state, slint_device_tx);
    midi_task(&rt, shutdown_rx.clone(), midi.clone(), midi_rx);

    let exp_dev = Rc::new(VecModel::from(vec![]));
    app.set_exposed_devices(ModelRc::from(Rc::clone(&exp_dev)));

    let _ = slint::spawn_local(async move {
        while let Ok(devices) = slint_device_rx.recv_async().await {
            //println!("devices {:?}", devices);
            let _ = &exp_dev.set_vec(
                devices
                    .iter()
                    .map(|d| format!("{}|{}|{}", d.cc, d.ui_type, d.description))
                    .map(SharedString::from)
                    .collect::<Vec<SharedString>>(),
            );
        }
    });

    // UI - MIDI
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

    let app_clone = app.clone_strong();
    let midi_clone = midi.clone();
    app.global::<AppState>().on_refresh_ports(move || {
        set_ports(app_clone.clone_strong(), midi_clone.clone());
    });

    let app_clone = app.clone_strong();
    let passthrough_clone = passthrough.clone();
    app.global::<AppState>().on_passthrough_click(move || {
        let app_clone = app_clone.clone_strong();
        let passthrough_clone = passthrough_clone.clone();
        let _ = slint::spawn_local(async move {
            let mut p = passthrough_clone.lock().await;
            *p = !*p;
            app_clone.global::<AppState>().set_passthrough(*p);
        });
    });

    // UI - EXPOSED DEVICES
    let device_tx_clone = device_tx.clone();
    app.global::<AppState>()
        .on_expose_device(move |cc, ui_type, description| {
            if let Some(d) = Device::from_string_args(
                cc.to_string(),
                ui_type.to_string(),
                description.to_string(),
            ) {
                let _ = device_tx_clone.send(DeviceCmd::Update(DeviceUpdate::Add(vec![d])));
            }
        });

    let device_tx_clone = device_tx.clone();
    app.global::<AppState>().on_hide_device(move |i| {
        if let Ok(i) = i.parse::<usize>() {
            let update = DeviceUpdate::Remove(vec![i]);
            let _ = device_tx_clone.send(DeviceCmd::Update(update));
        }
    });

    let device_tx_clone = device_tx.clone();
    app.global::<AppState>().on_paste(move || {
        let _ = device_tx_clone.send(DeviceCmd::Paste);
    });

    let device_tx_clone = device_tx.clone();
    app.global::<AppState>().on_clear_all(move || {
        let _ = device_tx_clone.send(DeviceCmd::Update(DeviceUpdate::Clear));
    });

    let device_tx_clone = device_tx.clone();
    app.global::<AppState>().on_copy_to_clipboard(move || {
        let _ = device_tx_clone.send(DeviceCmd::CopyToClipboard);
    });

    let device_tx_clone = device_tx.clone();
    let app_clone = app.clone_strong();
    app.global::<AppState>().on_login(move |url, pass| {
        set_ports(app_clone.clone_strong(), midi.clone());

        let login_payload = Login {
            url: url.to_string(),
            pass: pass.to_string(),
        };
        let _ = login_tx.send(login_payload.clone());

        let logged_in = login_response_rx.recv().unwrap_or(false);

        if logged_in {
            let _ = device_tx_clone.send(DeviceCmd::Login(login_payload));
        };

        app_clone.global::<AppState>().set_logged_in(logged_in);
    });

    // STOP
    let app_clone = app.clone_strong();
    let logout_tx_clone = logout_tx.clone();
    app.global::<AppState>().on_disconnect(move || {
        let _ = logout_tx_clone.send(());
        app_clone.global::<AppState>().set_logged_in(false);
    });
    let shutdown_tx_clone = shutdown_tx.clone();
    let logout_tx_clone = logout_tx.clone();
    app.window().on_close_requested(move || {
        let _ = logout_tx_clone.send(());
        let _ = shutdown_tx_clone.send(true);
        CloseRequestResponse::HideWindow
    });

    let _ = app.run();
    sleep(Duration::from_millis(100));
    Ok(())
}
