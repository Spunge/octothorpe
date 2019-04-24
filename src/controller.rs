
use super::message::Message;

#[derive(Debug)]
pub struct Controller {
    device_id: Option<u8>,
    pub buffer: Vec<Message>,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            device_id: None,
            buffer: Vec::new(),
        }
    }

    fn initialize(&mut self, device_id: u8) {
        self.device_id = Some(device_id);

        let message = Message::Introduction( 
            0, 
            [0xF0, 0x47, self.device_id.unwrap(), 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]
        );

        self.buffer.push(message);
    }

    fn key_pressed(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        match event.bytes[1] {
            91 => jack_client.transport_start(),
            92 => {
                 let (state, _) = jack_client.transport_query();
                 match state {
                    1 => jack_client.transport_stop(),
                    _ => {
                        let pos = jack::Position::default();
                        jack_client.transport_reposition(pos);
                    }
                 }
            },
            _ => return,
        };
    }

    fn key_released(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        
    }

    pub fn process_midi_event(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        // Sysex events pass us a lot of data
        // It's cleaner to check the first byte though
        if event.bytes.len() > 3 {
            self.process_sysex_message(event);
        } else {
            self.process_message(event, jack_client);
        }
    }

    fn process_sysex_message(&mut self, event: jack::RawMidi) {
        // 0x06 = inquiry message, 0x02 = inquiry response
        if event.bytes[3] == 0x06 && event.bytes[4] == 0x02  {
            // 0x47 = akai manufacturer, 0x73 = model nr
            if event.bytes[5] == 0x47 && event.bytes[6] == 0x73 {
                self.initialize(event.bytes[13]);
            }
        } else {
            println!("Got unknown sysex message");
            println!("{:?}", event);
        }
    }

    fn process_message(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        println!("0x{:X}, 0x{:X}, 0x{:X}", event.bytes[0], event.bytes[1], event.bytes[2]);
        println!("{}, {}, {}", event.bytes[0], event.bytes[1], event.bytes[2]);

        match event.bytes[0] {
            144 => self.key_pressed(event, jack_client),
            128 => self.key_released(event, jack_client),
            _ => {
                println!("Unknown event: {:?}", event);
            }
        }
    }
}

