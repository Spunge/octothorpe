
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;

//pub mod device;
pub mod hardware;
pub mod inputevent;

pub mod message;
pub mod sequencer;
pub mod cycle;
pub mod channel;
pub mod loopable;
pub mod sequence;
pub mod interface;
pub mod port;
//pub mod mixer;
pub mod events;
pub mod instrument;
pub mod router;
pub mod tickrange;
pub mod handler;
pub mod transport;
pub mod octothorpe;
pub mod device_manager;

// TODO - Save & load state on restart
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::fmt::Debug;
use sequencer::Sequencer;
use loopable::*;
use events::*;
//use device::*;
use channel::*;
use sequence::*;
use interface::*;
use message::*;
use hardware::*;
use port::*;
use cycle::*;
use router::*;
use tickrange::*;
use handler::*;
use transport::*;
use octothorpe::*;
use device_manager::*;

struct Offset {
    x: u8,
    y: u8,
}

fn main() {
    // Setup client
    let (client, _status) = jack::Client::new("octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();
    let (port_registration_sender, port_registration_receiver) = channel();

    let devices = Arc::new(Mutex::new(vec![]));
    let mut device_manager = DeviceManager::new(port_registration_receiver, Arc::clone(&devices));
    let octothorpe = Arc::new(Mutex::new(Octothorpe::new(devices)));

    //let notificationhandler = NotificationHandler::new(connection_send);
    let notificationhandler = NotificationHandler::new(port_registration_sender);
    let timebasehandler = TimebaseHandler::new(Arc::clone(&octothorpe));
    //let processhandler = ProcessHandler::new(introduction_receive, timebase_sender, &client);
    let processhandler = ProcessHandler::new(Arc::clone(&octothorpe));

    // Activate client
    let async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    device_manager.watch_registrations(async_client.as_client());
}

