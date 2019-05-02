
extern crate jack;

pub mod controller;
pub mod handlers;
pub mod message;
pub mod sequencer;
pub mod cycle;
pub mod instrument;
pub mod phrase;
pub mod pattern;
pub mod note;
pub mod grid;
pub mod sequence;
pub mod playable;

use std::io;
use std::sync::mpsc::channel;
use controller::Controller;
use handlers::*;

const TICKS_PER_BEAT: u32 = 1920;
const BEATS_PER_BAR: u32 = 4;

fn beats_to_ticks(beats: u32) -> u32 {
    beats * TICKS_PER_BEAT
}

fn bars_to_beats(bars: u32) -> u32 {
    bars * BEATS_PER_BAR
}

fn bars_to_ticks(bars: u32) -> u32 {
    bars_to_beats(bars) * TICKS_PER_BEAT
}

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (sender, receiver) = channel();

    let controller = Controller::new();

    let processhandler = ProcessHandler::new(controller, receiver, &client);
    let timebasehandler = TimebaseHandler::new();
    let notificationhandler = NotificationHandler::new(sender);

    // Activate client
    let async_client = client
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

    // Try to connect to synth & midi monitors for testing
    let _res = async_client.as_client().connect_ports_by_name("Octothorpe:midi_out", "amsynth:midi_in");
    let _res = async_client.as_client().connect_ports_by_name("Octothorpe:midi_out", "midi_out:input");
    let _res = async_client.as_client().connect_ports_by_name("Octothorpe:control_out", "control_out:input");

    // Wait for user to input string
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

