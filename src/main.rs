

extern crate jack;

use std::io;

pub mod controller;
pub mod scroller;
pub mod transport;

#[derive(Debug)]
pub enum RawMessage {
    Introduction([u8; 12]),
    Inquiry([u8; 6]),
    Note([u8; 3]),
}

#[derive(Debug)]
pub struct Message {
    time: u32,
    bytes: RawMessage,
}

impl Message {
    fn new(time: u32, message: RawMessage) -> Self {
        Message {
            time,
            bytes: message,
        }
    }
}

struct ProcessHandler {
    midi_out: jack::Port<jack::MidiOut>,
    midi_in: jack::Port<jack::MidiIn>,
    controller: controller::Controller,
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Process incoming midi
        for event in self.midi_in.iter(process_scope) {
            self.controller.process_midi_event(event);
        }

        // process outgoing midi
        let mut writer = self.midi_out.writer(process_scope);

        // Get buffer, output events, clear buffer
        for message in self.controller.get_midi_output() {
            match message.bytes {
                RawMessage::Introduction(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Inquiry(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Note(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
            }
        }

        // Clear buffer after writing events
        self.controller.clear_buffer();

        jack::Control::Continue
    }
}

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    // Create ports
    let midi_in = client
        .register_port("control_in", jack::MidiIn::default())
        .unwrap();
    let midi_out = client
        .register_port("control_out", jack::MidiOut::default())
        .unwrap();

    // Setup controller
    let controller = controller::Controller::new();

    // Setup process handler that will handle in / output
    let processhandler = ProcessHandler{
        midi_in: midi_in,
        midi_out: midi_out,
        controller: controller,
    };
    
    let transport = transport::Transport::new(120.0);

    // Activate client
    let active_client = client
        .activate_async((), processhandler, transport)
        .unwrap();

    // Get APC ports
    let ports = active_client.as_client().ports(Some("Akai APC40"), None, jack::PortFlags::empty());

    // Try to connect to APC automagically
    for port in ports {
        // TODO - Error handling of this
        if port.contains("capture") {
            active_client.as_client().connect_ports_by_name(&port, "Octothorpe:control_in");
        } else if port.contains("playback") {
            active_client.as_client().connect_ports_by_name("Octothorpe:control_out", &port);
        }
    };

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}

