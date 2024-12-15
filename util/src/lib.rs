mod exposed_devices;
pub use exposed_devices::{Device, DeviceCmd, DeviceUpdate, ExposedDevices, UIType};
mod midi;
pub use midi::{Midi, MidiCmd};

#[derive(Clone)]
pub struct Login {
    pub url: String,
    pub pass: String,
}
