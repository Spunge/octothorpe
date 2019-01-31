


pub struct Controller<'a> {
    //pub writer: jack::RingBufferWriter,
    is_identified: bool,
    buffer: Vec<&'a jack::RawMidi<'a>>,
}

impl<'a> Controller<'a> {
    pub fn new() -> Self {
        Controller {
            is_identified: false,
            buffer: Vec::new(),
        }
    }

    pub fn is_identified(&self) -> bool {
        self.is_identified
    }

    fn process_sysex_message(&self, event) {
    },

    fn process_message(&self, event) {
        println!("Got Midi!");
        println!("{:?}", event);
    },

    pub fn process_midi_event(&self, event: jack::RawMidi<'a>) {
        // Sysex events pass us a lot of data
        if event.bytes.len() > 3 {
            self.process_sysex_message(event)
        } else {
            self.process_message(event);
        }
    }

    pub fn get_midi_output(&self) -> &Vec<&jack::RawMidi<'a>> {
        &self.buffer
    }

    pub fn get_device_enquiry_request(&self) -> &jack::RawMidi<'a> {
        &jack::RawMidi{
            time: 0,
            bytes: &[0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7],
        }
    }

    // Try to identify connected controller
    pub fn identify(&self) {
        println!("One of my ports got connected, sending identify request");

        /*
        let event = jack::RawMidi {
            time: 0,
            bytes: &[
                0b10010000 /* Note On, channel 1 */, 0b01000000 /* Key number */,
                0b01111111 /* Velocity */,
            ],
        };
        */

        //self.output_buffer.push(event);
    }
}

