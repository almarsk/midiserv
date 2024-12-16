mod exposed_devices;
pub use exposed_devices::{Device, DeviceUpdate, UIType};
mod midi;
pub use midi::{Midi, MidiCmd};

#[derive(Clone, Debug)]
pub struct Login {
    pub url: String,
    pub pass: String,
}

#[derive(Debug)]
pub enum DeviceCmd {
    Login(Login),
    CopyToClipboard,
    Update(DeviceUpdate),
}
