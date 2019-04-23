

extern crate jack;

use std::io;
use std::sync::mpsc::channel;

pub mod client;
pub mod controller;
pub mod scroller;
pub mod handlers;

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

fn main() {
    // Setup client
    let (jack_client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (midi_sender, midi_receiver) = channel();
    let (bpm_sender, bpm_receiver) = channel();

    let client = client::Client::new(midi_sender.clone(), bpm_sender);

    let processhandler = handlers::ProcessHandler::new(midi_receiver, client, &jack_client);
    let timebasehandler = handlers::TimebaseHandler::new(bpm_receiver);
    let notificationhandler = handlers::NotificationHandler::new(midi_sender);

    // Activate client
    let async_client = jack_client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    // Get APC ports
    let ports = async_client.as_client().ports(Some("Akai APC40"), None, jack::PortFlags::empty());

    // Try to connect to APC automagically
    for port in ports {
        if port.contains("capture") {
            let _res = async_client.as_client().connect_ports_by_name(&port, "Octothorpe:control_in");
        } else if port.contains("playback") {
            let _res = async_client.as_client().connect_ports_by_name("Octothorpe:control_out", &port);
        }
    };

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

