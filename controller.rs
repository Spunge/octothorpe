


pub struct Controller {
    //pub writer: jack::RingBufferWriter,
    identified: bool,
    buffer: Vec<jack::RawMidi>,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            identified: false,
            buffer: Vec::new(),
        }
    }

    pub fn get_midi_output(&self) {
        if ! self.identifed {
            vec![self.get_device_enquiry_request()]
        } else {
            &self.buffer
        }
    },

    fn get_device_enquiry_request(&self) {
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

