
extern crate jack;

use std::io;

mod controller;

struct NotificationHandler {
    controller: controller::Controller,
}

impl jack::NotificationHandler for NotificationHandler {
    fn ports_connected(&mut self, client: &jack::Client, port_id_a: jack::PortId, port_id_b: jack::PortId, are_connected: bool) {
        // Get ports from client
        let port_a = client.port_by_id(port_id_a).unwrap();
        let port_b = client.port_by_id(port_id_b).unwrap();

        // Only interested in our own ports
        if (client.is_mine(&port_a) || client.is_mine(&port_b)) && are_connected {
            println!("One of client ports got connected, sending identify request");

            self.controller.identify();
        }
    }
}

struct ProcessHandler {
    midi_out: jack::Port<jack::MidiOut>,
    midi_in: jack::Port<jack::MidiIn>,
    //reader: jack::RingBufferReader,
}
impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, _client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Process incoming midi
        for event in self.midi_in.iter(process_scope) {
            println!("Got Midi!");
            println!("{:?}", event);
        }

        // Read from output buffer and output those events
        if self.reader.space() != 0 {
            // TODO - start reading till 0xF7 occurs, that will be a message, if space is larger,
            // do this again
            
            //let mut outbuf = [0u8; 6];
            //self.reader.read_buffer(&mut outbuf);

            let mut put_p = self.midi_out.writer(process_scope);

            println!("Sending Midi!");
            println!("{:?}", outbuf);
            put_p.write(&jack::RawMidi { time: 0, bytes: &outbuf }).unwrap();
        }

        jack::Control::Continue
    }
}

fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    //let output_buffer = jack::RingBuffer::new(1024).unwrap();
    //let (output_reader, output_writer) = output_buffer.into_reader_writer();

    // Create ports
    let midi_in = client
        .register_port("control_in", jack::MidiIn::default())
        .unwrap();
    let midi_out = client
        .register_port("control_out", jack::MidiOut::default())
        .unwrap();

    // Setup controller
    let controller = controller::Controller{
        //writer: output_writer,
    };

    // Setup handlers
    let processhandler = ProcessHandler{
        midi_in: midi_in,
        midi_out: midi_out,
        //reader: output_reader,
        controller: controller,
    };

    // Activate client
    let active_client = client
        .activate_async((), processhandler)
        .unwrap();

    // Wait for user to input string
    println!("Press any key to quit");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();

    // Clean up
    active_client.deactivate().unwrap();
}

