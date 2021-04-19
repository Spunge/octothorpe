
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

// This will keep track of what controllers are connected
struct ControllerManager {
    controllers: Arc<Mutex<Vec<Controller>>>,
    port_registration_receiver: Receiver<(jack::PortId, bool)>,
    registered_ports: Vec<jack::Port<jack::Unowned>>,
}

impl ControllerManager {
    pub fn new(port_registration_receiver: Receiver<(jack::PortId, bool)>, controllers: Arc<Mutex<Vec<Controller>>>) -> Self {
        Self { 
            controllers,
            port_registration_receiver,
            registered_ports: vec![],
        }
    }

    // TODO - We can't unwrap anything that's jack related without handling errors
    // Create controller representations when controllers connect
    pub fn register_port(&mut self, port: jack::Port<jack::Unowned>, client: &jack::Client) {
        // Create new controller representation when input&output port both registered
        if port.flags().contains(jack::PortFlags::IS_OUTPUT) {
            let aliases = port.aliases().unwrap();

            // Remember system capture port registered
            if let Some(alias) = aliases.iter().find(|alias| alias.contains("APC40") || alias.contains("APC20")) {
                self.registered_ports.push(port);
            }
        } else {
            // Sink port registered, add it to correct controller
            let capture_port_name = port.name().unwrap().replace("playback", "capture");

            // Got input port aswell?
            let capture_port = self.registered_ports.iter_mut().enumerate().find(|(_index, port)| port.name().unwrap() == capture_port_name);
            if let Some((index, _port)) = capture_port {
                let capture_port = self.registered_ports.swap_remove(index);

                // Add found controller
                let is_apc40 = port.aliases().unwrap().iter().find(|alias| alias.contains("APC40")).is_some();
                let controller_type = if is_apc40 { APC::new(APC40::new()) } else { APC::new(APC20::new()) };

                //println!("adding controller {:?}", capture_port.aliases().unwrap().first().unwrap());
                self.controllers.lock().unwrap().push(Controller::new(client, capture_port, port, controller_type));
            }
        }
    }

    // Destroy controller when ports disconnect
    pub fn deregister_port(&mut self, port: jack::Port<jack::Unowned>, client: &jack::Client) {
        let mut controllers = self.controllers.lock().unwrap();

        // Only destroy on output port disconnect as controllers have multiple ports
        let controller = controllers.iter().enumerate()
            .find(|(_index, controller)| controller.system_source.name().unwrap() == port.name().unwrap());

        // Deregister jack ports on removing controller
        if let Some((index, controller)) = controller {
            //println!("removing controller {:?}", &controller.system_source.name().unwrap());
            // We get a new port representation as we need to own port that we pass to client,
            // and we can't own octo as it sits behind mutex
            let input_port = client.port_by_name(&controller.input.name().unwrap()).unwrap();
            client.unregister_port(input_port);
            let output_port = client.port_by_name(&controller.output.name().unwrap()).unwrap();
            client.unregister_port(output_port);

            // Remove controller from octo
            controllers.swap_remove(index);
        }
    }

    pub fn watch_registrations(&mut self, client: &jack::Client) {
        // Currently existing registered ports should also be handled
        let ports: Vec<jack::Port<jack::Unowned>> = client
            .ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL)
            .into_iter()
            .map(|name| client.port_by_name(&name).unwrap())
            .collect();

        for port in ports {
            self.register_port(port, client);
        }

        // Wait for notifications about new ports
        while let Ok((port_id, is_registered)) = self.port_registration_receiver.recv() {
            let port = client.port_by_id(port_id).unwrap();

            // We're not interested in ports we are creating ourselves
            if client.is_mine(&port) {
                continue
            }
            
            if is_registered {
                self.register_port(port, client);
            } else {
                self.deregister_port(port, client);
            }
        }
    }
}

fn main() {
    // Setup client
    let (client, _status) = jack::Client::new("octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();
    let (port_registration_sender, port_registration_receiver) = channel();

    let octothorpe = Arc::new(Mutex::new(Octothorpe::new()));
    let controllers = Arc::new(Mutex::new(vec![]));
    let mut controller_manager = ControllerManager::new(port_registration_receiver, Arc::clone(&controllers));

    //let notificationhandler = NotificationHandler::new(connection_send);
    let notificationhandler = NotificationHandler::new(port_registration_sender);
    let timebasehandler = TimebaseHandler::new(Arc::clone(&octothorpe));
    //let processhandler = ProcessHandler::new(introduction_receive, timebase_sender, &client);
    let processhandler = ProcessHandler::new(Arc::clone(&controllers), Arc::clone(&octothorpe));

    // Activate client
    let async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    controller_manager.watch_registrations(async_client.as_client());
}

