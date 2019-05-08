
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
pub mod sequence;
pub mod playable;

use std::io;
use std::sync::mpsc::channel;
use controller::Controller;
use handlers::*;

const TICKS_PER_BEAT: u32 = 1920;
const BEATS_PER_BAR: u32 = 4;

fn beats_to_ticks(beats: f64) -> u32 {
    (beats * TICKS_PER_BEAT as f64) as u32
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
    let _async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    // Wait for user to input string
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

