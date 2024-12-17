mod exposed_devices;
use clipboard::{ClipboardContext, ClipboardProvider};
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
    Paste,
}

pub fn get_clipboard_content() -> Option<String> {
    ClipboardProvider::new()
        .ok()
        .map(|ctx| {
            let mut ctx: ClipboardContext = ctx;
            ctx.get_contents().ok()
        })
        .unwrap_or(None)
}

pub fn copy_to_clipboard(content: String) {
    ClipboardProvider::new()
        .ok()
        .and_then(|mut ctx: ClipboardContext| ctx.set_contents(content).ok());
}
