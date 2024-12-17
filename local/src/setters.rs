use crate::{AppState, AppWindow};
use flume::{Receiver, Sender};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;
use util::{DeviceCmd, DeviceUpdate, Midi, UIType};

pub fn set_ports(app: AppWindow, midi: Arc<Mutex<Midi>>) {
    let ports = Rc::new(
        midi.blocking_lock()
            .get_ports()
            .iter()
            .map(SharedString::from)
            .collect::<VecModel<_>>(),
    );
    app.global::<AppState>()
        .set_midi_ports(ModelRc::from(Rc::clone(&ports)));
}

pub fn init_ui_types(app: AppWindow) {
    let ui_types = Rc::new(VecModel::from(
        UIType::to_vec()
            .iter()
            .map(SharedString::from)
            .collect::<Vec<SharedString>>(),
    ));
    app.set_ui_types(ModelRc::from(Rc::clone(&ui_types)));
}

pub enum Status {
    Connection(bool),
    Text(String),
}

pub fn connection_status(
    app: AppWindow,
    status_rx: Receiver<Status>,
    device_tx: Sender<DeviceCmd>,
) {
    let _ = slint::spawn_local(async move {
        let app_state = app.global::<AppState>();
        loop {
            if let Ok(status) = status_rx.recv_async().await {
                match status {
                    Status::Connection(s) => {
                        if !s {
                            let _ = device_tx.send_async(DeviceCmd::Update(DeviceUpdate::Clear));
                        }
                        app_state.set_connected_to_server(s)
                    }
                    Status::Text(t) => app_state.set_server_name(SharedString::from(t)),
                }
            };
        }
    });
}
