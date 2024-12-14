use std::sync::Arc;

use flume::Receiver;
use tokio::{runtime::Runtime, sync::Mutex};
use util::{Midi, MidiCmd};

pub fn midi_task(
    rt: &Runtime,
    shutdown: Receiver<bool>,
    midi: Arc<Mutex<Midi>>,
    midi_rx: Receiver<MidiCmd>,
) {
    rt.spawn(async move {
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
                shutdown_option = shutdown.recv_async() => {
                    if let Ok(shutdown) = shutdown_option {
                        if shutdown {
                            break;
                        }
                    }
                }
            }
        }
    });
}
