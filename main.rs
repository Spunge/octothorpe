
extern crate jack;

use std::io;

mod controller;
mod handlers;

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    // Create ports
    let midi_in = client
        .register_port("control_in", jack::MidiIn::default())
        .unwrap();
    let mut midi_out = client
        .register_port("control_out", jack::MidiOut::default())
        .unwrap();

    // Setup controller
    let controller = controller::Controller::new();

    // Setup handlers
    let processhandler = handlers::ProcessHandler { 
        controller: &controller,
        midi_in: &midi_in,
        midi_out: &mut midi_out,
    };
    let notificationhandler = handlers::NotificationHandler {
        controller: &controller,
    };

    // Activate client
    let active_client = client
        .activate_async(notificationhandler, processhandler)
        .unwrap();

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}

