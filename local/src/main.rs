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
use setters::init_exposed_devices;
use setters::init_ui_types;
use setters::set_ports;
use slint::CloseRequestResponse;
use slint::ComponentHandle;
use slint::{SharedString, VecModel};
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use tasks::device_task;
use tasks::login_task;
use tasks::logout_task;
use tasks::midi_task;
use tokio::sync::Mutex;
use util::Device;
use util::DeviceCmd;
use util::Midi;
use util::MidiCmd;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // INIT
    let app = AppWindow::new()?;
    set_ports(app.clone_strong());
    let exp_dev = init_exposed_devices(app.clone_strong());
    init_ui_types(app.clone_strong());

    let login: Arc<Mutex<Option<Login>>> = Arc::new(Mutex::new(None));
    let rt = tokio::runtime::Runtime::new().unwrap();

    // CHANNELS
    let (shutdown_tx, shutdown_rx): (Sender<bool>, Receiver<bool>) = bounded(10);
    let (midi_tx, midi_rx): (Sender<MidiCmd>, Receiver<MidiCmd>) = bounded(10);
    let (dvc_tx, dvc_rx): (Sender<DeviceCmd>, Receiver<DeviceCmd>) = bounded(10);
    let (dvc_rpns_tx, dvc_rpns_rx): (Sender<Vec<String>>, Receiver<Vec<String>>) = bounded(10);
    let (login_tx, login_rx): (Sender<Login>, Receiver<Login>) = bounded(10);
    let (logout_tx, logout_rx): (Sender<()>, Receiver<()>) = bounded(10);
    let (login_response_tx, login_response_rx): (Sender<bool>, Receiver<bool>) = bounded(10);
    let (status_tx, status_rx): (Sender<Status>, Receiver<Status>) = bounded(10);

    // EXPOSED DEVICES
    let state = Rc::new(ExposedState {
        tx: dvc_tx.clone(),
        rx: dvc_rpns_rx.clone(),
        exp_dev,
    });

    // TASKS
    logout_task(&rt, shutdown_rx.clone(), logout_rx.clone(), login.clone());

    let app_clone = app.clone_strong();
    update_connection_status(app_clone, status_rx);
    login_task(
        &rt,
        shutdown_rx.clone(),
        login_rx,
        midi_tx.clone(),
        login_response_tx.clone(),
        login.clone(),
        status_tx,
    );
    device_task(&rt, shutdown_rx.clone(), dvc_rx, dvc_rpns_tx);
    midi_task(&rt, shutdown_rx.clone(), Midi::new(), midi_rx);

    // UI
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

    let app_clone = app.clone_strong();
    app.global::<AppState>().on_refresh_ports(move || {
        set_ports(app_clone.clone_strong());
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
        set_ports(app_clone.clone_strong());
        let _ = login_tx.send(Login {
            url: url.to_string(),
            pass: pass.to_string(),
        });

        app_clone
            .global::<AppState>()
            .set_logged_in(login_response_rx.recv().unwrap_or(false));
    });

    let app_clone = app.clone_strong();
    let logout_tx_clone = logout_tx.clone();
    app.global::<AppState>().on_disconnect(move || {
        let _ = logout_tx_clone.send(());
        app_clone.global::<AppState>().set_logged_in(false);
    });

    // STOP
    let logout_tx_clone = logout_tx.clone();
    app.window().on_close_requested(move || {
        let _ = logout_tx_clone.send(());
        let _ = shutdown_tx.send(true);
        CloseRequestResponse::HideWindow
    });

    run_app(app, rt)
}

fn run_app(app: AppWindow, rt: tokio::runtime::Runtime) -> Result<(), Box<dyn std::error::Error>> {
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

enum Status {
    Connection(bool),
    Text(String),
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

fn update_connection_status(app: AppWindow, status_rx: Receiver<Status>) {
    let _ = slint::spawn_local(async move {
        let app_state = app.global::<AppState>();
        loop {
            if let Ok(status) = status_rx.recv_async().await {
                match status {
                    Status::Connection(s) => app_state.set_connected_to_server(s),
                    Status::Text(t) => app_state.set_server_name(SharedString::from(t)),
                }
            };
        }
    });
}
