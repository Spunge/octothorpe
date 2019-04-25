
use super::message::Message;
use super::sequencer::Sequencer;
use super::handlers::Writer;

pub struct Controller {
    pub sequencer: Sequencer,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            sequencer: Sequencer::new(),
        }
    }

    fn key_pressed(&mut self, event: jack::RawMidi, client: &jack::Client) {
        match event.bytes[1] {
            91 => {
                println!("Starting transport \n");
                client.transport_start();
            },
            92 => {
                 let (state, _) = client.transport_query();
                 match state {
                    1 => {
                        println!("Stopping transport \n");
                        client.transport_stop();
                    },
                    _ => {
                        println!("Ressetting transport \n");
                        let pos = jack::Position::default();
                        client.transport_reposition(pos);
                    }
                 }
            },
            _ => return,
        };
    }

    fn key_released(&mut self, _event: jack::RawMidi, _client: &jack::Client) {
        
    }

    pub fn process_midi_event(
        &mut self,
        event: jack::RawMidi,
        client: &jack::Client,
        control_out: &mut Writer,
    ) {
        // Sysex events pass us a lot of data
        // It's cleaner to check the first byte though
        if event.bytes.len() > 3 {
            self.process_sysex_message(event, control_out)
        } else {
            self.process_message(event, client, control_out)
        }
    }

    fn process_sysex_message(&mut self, event: jack::RawMidi, control_out: &mut Writer) {
        // 0x06 = inquiry message, 0x02 = inquiry response
        if event.bytes[3] == 0x06 && event.bytes[4] == 0x02  {
            // 0x47 = akai manufacturer, 0x73 = model nr
            if event.bytes[5] == 0x47 && event.bytes[6] == 0x73 {
                // Introduce ourselves to controller
                let message = Message::Introduction( 
                    0, 
                    [0xF0, 0x47, event.bytes[13], 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7],
                );

                control_out.write(message);
            }
        } else {
            println!("Got unknown sysex message");
            println!("{:?}", event);
        }
    }

    fn process_message(&mut self, event: jack::RawMidi, client: &jack::Client, _control_out: &mut Writer) {
        // Output in hex so we can compare to apc40 manual easily
        println!("0x{:X}, 0x{:X}, 0x{:X}", event.bytes[0], event.bytes[1], event.bytes[2]);
        println!("{}, {}, {}", event.bytes[0], event.bytes[1], event.bytes[2]);

        match event.bytes[0] {
            144 => self.key_pressed(event, client),
            128 => self.key_released(event, client),
            _ => {},
        }
    }
}

