use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};

// inspired by https://github.com/Boddlnagg/midir/blob/master/examples/test_play.rs

const CC_MESSAGE: u8 = 0xB0;

pub struct Midi {
    conn: Option<MidiOutputConnection>,
    ports: Vec<Port>,
}

pub enum MidiCommand {
    Dummy(u8),
    Port(usize),
}

struct Port {
    id: String,
    name: String,
    port: MidiOutputPort,
}

impl Port {
    fn new(id: String, name: String, port: MidiOutputPort) -> Self {
        Port { id, name, port }
    }
}

impl Midi {
    pub fn new() -> Self {
        MidiOutput::new("midiserve")
            .ok()
            .map(|midi_output| Midi {
                conn: None,
                ports: midi_output
                    .ports()
                    .into_iter()
                    .map(|p| {
                        Port::new(
                            p.id().clone(),
                            midi_output.port_name(&p).unwrap_or_else(|_| "".to_string()),
                            p,
                        )
                    })
                    .collect(),
            })
            .unwrap_or_else(|| panic!())
    }

    pub fn get_ports(&mut self) -> Vec<String> {
        self.ports
            .iter()
            .map(|p| format!("{}|{}", p.id.clone(), p.name.clone()))
            .collect()
    }

    pub fn update_port(&mut self, out_port: usize) {
        self.conn = self.ports.get(out_port).and_then(|p| {
            MidiOutput::new("midiserve")
                .ok()
                .and_then(|m| m.connect(&p.port, "midiserv").ok())
        });
    }

    pub fn send_cc(&mut self, controller: u8, value: u8) {
        self.conn
            .as_mut()
            .and_then(|c| c.send(&[CC_MESSAGE, controller, value]).ok());
    }
}
