
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;

pub mod controller;
pub mod hardware;
pub mod message;
pub mod sequencer;
pub mod cycle;
pub mod channel;
pub mod loopable;
pub mod sequence;
pub mod surface;
pub mod port;
pub mod mixer;
pub mod events;
pub mod instrument;
pub mod router;
pub mod tickrange;
pub mod handler;

// TODO - Save & load state on restart
use std::io;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::fmt::Debug;
use sequencer::Sequencer;
use loopable::*;
use events::*;
use controller::*;
use surface::*;
use hardware::*;
use port::*;
use surface::Surface;
use cycle::*;
use router::*;
use tickrange::*;
use handler::*;

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let surface = Arc::new(Mutex::new(Surface::new()));

    let (port_registration_sender, port_registration_receiver) = channel();

    //let notificationhandler = NotificationHandler::new(connection_send);
    let notificationhandler = NotificationHandler::new(&client, Arc::clone(&surface), port_registration_sender);
    let timebasehandler = TimebaseHandler::new();
    //let processhandler = ProcessHandler::new(introduction_receive, timebase_sender, &client);
    let processhandler = ProcessHandler::new(Arc::clone(&surface));

    // Activate client
    let async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();


    let client = async_client.as_client();

    // Wait for notifications about new ports
    while let Ok((port, is_registered)) = port_registration_receiver.recv() {
        // We're not interested in ports we are creating ourselves, as connections to correct ports
        // should be handled by controller
        if client.is_mine(&port) {
            continue
        }

        let mut surface = surface.lock().unwrap();

        if is_registered {
            // Create new controller representations in surface when ports register
            // It seems like output ports are always reported first, so we create controller when
            // this port registers, and add the sink to existing controller based on port name
            if port.flags().contains(jack::PortFlags::IS_OUTPUT) {
                let aliases = port.aliases().unwrap();

                // Add controller if port has an alias we know
                if aliases.iter().find(|alias| alias.contains("APC40")).is_some() {
                    surface.controllers.push(Controller::new(port, APC40::new(client)));
                } else if aliases.iter().find(|alias| alias.contains("APC20")).is_some() {
                    surface.controllers.push(Controller::new(port, APC20::new(client)));
                }
            } else {
                // Sink port registered, add it to correct controller
                let capture_port_name = port.name().unwrap().replace("playback", "capture");

                let controller = surface.controllers.iter_mut()
                    .find(|controller| controller.system_source.name().unwrap() == capture_port_name);

                if controller.is_some() {
                    controller.unwrap().system_sink = Some(port);
                }
            }
        } else {
            // Destroy controller in surface when ports disconnect
            // Only destroy on output port disconnect as controllers have multiple ports
            println!("{:?}", surface.controllers.len());
            // TODO - Deregister ports on removing controller
            surface.controllers.retain(|controller| controller.system_source.name().unwrap() != port.name().unwrap());
            println!("{:?}", surface.controllers.len());
        }


        //if aliases.iter().find(|alias| alias.contains("APC40")).is_some() {
        //println!("APC 40 {:?}", if is_registered { "CONNECTED" } else { "DISCONNECTED" });
        //}

        //if aliases.iter().find(|alias| alias.contains("APC20")).is_some() {
        //println!("APC 20 {:?}", if is_registered { "CONNECTED" } else { "DISCONNECTED" });
        //}

        //println!("{:?}", port);
        //println!("{:?}", is_registered);

    }

    // Start router that will listen for new ports & handle connections
    //let mut user_input = String::new();
    //io::stdin().read_line(&mut user_input).ok();
}

