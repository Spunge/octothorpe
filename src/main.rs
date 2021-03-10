
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;

//pub mod controller;
pub mod hardware;
pub mod inputevent;

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
pub mod transport;

// TODO - Save & load state on restart
use std::sync::{Arc, Mutex};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::fmt::Debug;
use sequencer::Sequencer;
use loopable::*;
use events::*;
//use controller::*;
use channel::*;
use sequence::*;
use surface::*;
use message::*;
use hardware::*;
use port::*;
use surface::Surface;
use cycle::*;
use router::*;
use tickrange::*;
use handler::*;
use transport::*;

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let surface = Arc::new(Mutex::new(Surface::new()));
    let transport = Arc::new(Mutex::new(Transport::new()));

    let (port_registration_sender, port_registration_receiver) = channel();

    //let notificationhandler = NotificationHandler::new(connection_send);
    let notificationhandler = NotificationHandler::new(&client, port_registration_sender);
    let timebasehandler = TimebaseHandler::new(Arc::clone(&transport));
    //let processhandler = ProcessHandler::new(introduction_receive, timebase_sender, &client);
    let processhandler = ProcessHandler::new(Arc::clone(&surface), Arc::clone(&transport));

    // Activate client
    let async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();


    let client = async_client.as_client();

    // Wait for notifications about new ports
    while let Ok((port, is_registered)) = port_registration_receiver.recv() {
        // We're not interested in ports we are creating ourselves
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
                    surface.controllers.push(Controller::new(port, client, APC::new(APC40::new())));
                } else if aliases.iter().find(|alias| alias.contains("APC20")).is_some() {
                    surface.controllers.push(Controller::new(port, client, APC::new(APC20::new())));
                }
            } else {
                // Sink port registered, add it to correct controller
                let capture_port_name = port.name().unwrap().replace("playback", "capture");

                let controller = surface.controllers.iter_mut()
                    .find(|controller| controller.system_source.name().unwrap() == capture_port_name);

                // Now that we've added sink port, controller is ready to communicate
                // Connect it to system ports of said controller
                if let Some(controller) = controller {
                    controller.system_sink = Some(port);
                    client.connect_ports(&controller.system_source, &controller.input);
                    client.connect_ports(&controller.output, &controller.system_sink.as_ref().unwrap());
                }
            }
        } else {
            // Destroy controller in surface when ports disconnect
            // Only destroy on output port disconnect as controllers have multiple ports
            let controller = surface.controllers.iter()
                .find(|controller| controller.system_source.name().unwrap() == port.name().unwrap());

            // Deregister jack ports on removing controller
            if let Some(controller) = controller {
                // We get a new port representation as we need to own port that we pass to client,
                // and we can't own surface as it sits behind mutex
                let input_port = client.port_by_name(&controller.input.name().unwrap()).unwrap();
                client.unregister_port(input_port);
                let output_port = client.port_by_name(&controller.output.name().unwrap()).unwrap();
                client.unregister_port(output_port);
            }

            surface.controllers.retain(|controller| controller.system_source.name().unwrap() != port.name().unwrap());
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

