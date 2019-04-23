

extern crate jack;

use std::io;

pub mod client;
pub mod controller;
pub mod scroller;

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
    //let _controller = controller::Controller::new();

    // Setup client
    let (jack_client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let client = client::Client::new(&jack_client);

    // Activate client
    let async_client = jack_client
        .activate_async((), client, client)
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

    async_client.deactivate().unwrap();
}

