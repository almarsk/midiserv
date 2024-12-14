use crate::{AppState, AppWindow};
use flume::{Receiver, Sender};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use util::{DeviceCmd, Midi, UIType};

pub fn set_ports(app: AppWindow) {
    let ports = Rc::new(
        Midi::new()
            .get_ports()
            .iter()
            .map(|s| SharedString::from(s))
            .collect::<VecModel<_>>(),
    );
    app.global::<AppState>()
        .set_midi_ports(ModelRc::from(Rc::clone(&ports)));
}

pub fn init_exposed_devices(app: AppWindow) -> Rc<VecModel<SharedString>> {
    let exp_dev = Rc::new(VecModel::from(vec![]));
    app.set_exposed_devices(ModelRc::from(Rc::clone(&exp_dev)));
    return exp_dev;
}

pub fn init_ui_types(app: AppWindow) {
    let ui_types = Rc::new(VecModel::from(
        UIType::to_vec()
            .iter()
            .map(|s| SharedString::from(s))
            .collect::<Vec<SharedString>>(),
    ));
    app.set_ui_types(ModelRc::from(Rc::clone(&ui_types)));
}

pub struct ExposedState {
    pub tx: Sender<DeviceCmd>,
    pub rx: Receiver<Vec<String>>,
    pub exp_dev: Rc<VecModel<SharedString>>,
}

pub fn update_exp_dev(state: Rc<ExposedState>) {
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

pub enum Status {
    Connection(bool),
    Text(String),
}

pub fn update_connection_status(app: AppWindow, status_rx: Receiver<Status>) {
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
