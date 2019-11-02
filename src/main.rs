
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

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (notification_sender, notification_receiver) = channel();
    let (timebase_sender, timebase_receiver) = channel();

    // TODO - Pass client to cotroller
    let controller = Controller::new(/*&client*/);

    let processhandler = ProcessHandler::new(controller, notification_receiver, timebase_sender, &client);
    let timebasehandler = TimebaseHandler::new(timebase_receiver);
    let notificationhandler = NotificationHandler::new(notification_sender);

    // Activate client
    let _async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    // Wait for user to input string
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

