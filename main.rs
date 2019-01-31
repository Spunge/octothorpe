
extern crate jack;

use std::io;

struct NotificationHandler {}

impl jack::NotificationHandler for NotificationHandler {
    fn ports_connected(&mut self, client: &jack::Client, port_id_a: jack::PortId, port_id_b: jack::PortId, are_connected: bool) {
        // Get ports from client
        let port_a = client.port_by_id(port_id_a).unwrap();
        let port_b = client.port_by_id(port_id_b).unwrap();

        // Only interested in our own ports
        if (client.is_mine(&port_a) || client.is_mine(&port_b)) && are_connected {
            println!("One of client ports got connected, sending identify request");

            // TODO - send device enquiry
            //let message = [0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7];
        }
    }
}

struct ProcessHandler {
    midi_in: jack::Port<jack::MidiIn>,
}
impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        for event in self.midi_in.iter(process_scope) {
            println!("{:?}", event);
        }
        // Handle input & output in controller
        //self.controller.process_midi_input(self.midi_in.iter(process_scope));
        //self.controller.write_midi_output(self.midi_out.writer(process_scope));

        jack::Control::Continue
    }
}

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    // Create ports
    let midi_in = client
        .register_port("control_in", jack::MidiIn::default())
        .unwrap();
    let mut midi_out = client
        .register_port("control_out", jack::MidiOut::default())
        .unwrap();

    // Setup controller
    //let controller = controller::Controller::new(writer);

    // Setup handlers
    let processhandler = ProcessHandler{
        midi_in: midi_in,
    };
    let notificationhandler = NotificationHandler{};

    // Activate client
    let active_client = client
        .activate_async(notificationhandler, processhandler)
        .unwrap();

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}

