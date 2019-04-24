

extern crate jack;

pub mod controller;
pub mod scroller;
pub mod handlers;
pub mod message;

use std::io;
use std::sync::mpsc::channel;
use controller::Controller;
use handlers::*;

fn main() {
    // Setup client
    let (jack_client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (sender, receiver) = channel();

    let controller = Controller::new();

    let processhandler = ProcessHandler::new(controller, receiver, &jack_client);
    let timebasehandler = TimebaseHandler::new();
    let notificationhandler = NotificationHandler::new(sender);

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
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

