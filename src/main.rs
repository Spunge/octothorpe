

extern crate jack;

use std::io;

mod controller;

pub enum Message {
    Introduction(u32, [u8; 12]),
    Inquiry(u32, [u8; 6]),
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
        for i in self.controller.get_midi_output()

            // TODO - output midi

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

    // Setup handlers
    let processhandler = ProcessHandler{
        midi_in: midi_in,
        midi_out: midi_out,
        controller: controller,
    };

    // Activate client
    let active_client = client
        .activate_async((), processhandler)
        .unwrap();

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}
