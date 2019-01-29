
extern crate jack;

use std::io;

mod controller;

// Listen for port connections to send hello message
struct ClosureNotificationsHandler<F: Send + FnMut(&jack::Client, jack::PortId, jack::PortId, bool)> {
    pub ports_connected_fn: F,
}

impl<F> ClosureNotificationsHandler<F>
where
    F: Send + FnMut(&jack::Client, jack::PortId, jack::PortId, bool),
{
    pub fn new(f: F) -> ClosureNotificationsHandler<F> {
        ClosureNotificationsHandler { ports_connected_fn: f }
    }
}

impl<F> jack::NotificationHandler for ClosureNotificationsHandler<F>
where
    F: Send + FnMut(&jack::Client, jack::PortId, jack::PortId, bool),
{
    fn ports_connected(&mut self, client: &jack::Client, port_id_a: jack::PortId, port_id_b: jack::PortId, are_connected: bool) {
        (self.ports_connected_fn)(client, port_id_a, port_id_b, are_connected);
    }
}

// Setup jack midi in & out ports
fn setup_ports(client: &jack::Client) -> (jack::Port<jack::MidiIn>, jack::Port<jack::MidiOut>) {
    println!("Setting up midi input & output ports");

    // Create ports
    let midi_in = client
        .register_port("control_in", jack::MidiIn::default())
        .unwrap();
    // Midi out should be mutable as we want to write to it
    //let mut midi_out = client
    let midi_out = client
        .register_port("control_out", jack::MidiOut::default())
        .unwrap();

    return (midi_in, midi_out);
}

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    // Get ports
    let (input, mut output) = setup_ports(&client);

    // Setup controller
    let controller = controller::Controller::new();

    // Jack process handler
    let process = |_: &jack::Client, process_scope: &jack::ProcessScope| -> jack::Control {
        let input_iterator = input.iter(process_scope);

        for event in input_iterator {
            println!("{:?}", event);
        }

        controller.write_midi_events(output.writer(process_scope));

        jack::Control::Continue
    };

    // Jack ports connected handler
    let ports_connected = |client: &jack::Client, port_id_a: jack::PortId, port_id_b: jack::PortId, are_connected: bool| {
        // Get ports from client
        let port_a = client.port_by_id(port_id_a).unwrap();
        let port_b = client.port_by_id(port_id_b).unwrap();

        // Only interested in our own ports
        if (client.is_mine(&port_a) || client.is_mine(&port_b)) && are_connected {
            // Make controller say hello
            controller.identify();
        }
    };

    // Activate client
    let active_client = client
        .activate_async(ClosureNotificationsHandler::new(ports_connected), jack::ClosureProcessHandler::new(process))
        .unwrap();

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}

