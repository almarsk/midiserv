use crate::{AppState, AppWindow};
use slint::ComponentHandle;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};
use util::{Midi, UIType};

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
