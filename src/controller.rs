

pub struct Controller {
    //pub writer: jack::RingBufferWriter,
    is_identified: bool,
    device_id: u8,
    buffer: Vec<super::Message>,
}

impl<'a> Controller {
    pub fn new() -> Self {
        Controller {
            is_identified: false,
            device_id: 0,
            buffer: Vec::new(),
        }
    }

    fn is_identified(&self) -> bool {
        self.is_identified
    }

    fn inquire(&mut self) {
        self.buffer.push(super::Message::new(
            0, 
            super::RawMessage::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]),
        ));
    }

    fn identify(&mut self, inquiry_response: jack::RawMidi<'a>) {
        // 0x47 = akai manufacturer, 0x73 = model nr
        if inquiry_response.bytes[5] == 0x47 && inquiry_response.bytes[6] == 0x73  {
            println!("Identified APC40");

            self.is_identified = true;
            self.device_id = inquiry_response.bytes[13];

            self.buffer.push(super::Message::new(
                0,
                super::RawMessage::Introduction([0xF0, 0x47, self.device_id, 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]),
            ));
            for x in 0..8 {
                 for y in 0..5 {
                    self.buffer.push(super::Message::new(
                        0,
                        super::RawMessage::Note([0x90 + x, 0x35 + y, 0x04]),
                    ));
                 }
            }
        }
    }

    fn process_sysex_message(&mut self, event: jack::RawMidi<'a>) {
        // 0x06 = inquiry message, 0x02 = inquiry response
        if event.bytes[3] == 0x06 && event.bytes[4] == 0x02  {
            println!("Got inquiry response!");
            self.identify(event);
        } else {
            println!("Got Sysex!");
        }
    }

    fn process_message(&self, event: jack::RawMidi<'a>) {
        println!("Got Midi!");
        println!("{:?}", event);
    }

    pub fn process_midi_event(&mut self, event: jack::RawMidi<'a>) {
        // Sysex events pass us a lot of data
        // It's cleaner to check the first byte though
        if event.bytes.len() > 3 {
            self.process_sysex_message(event)
        } else {
            self.process_message(event);
        }
    }

    pub fn get_midi_output(&mut self) -> &Vec<super::Message> {
        if ! self.is_identified {
            self.inquire();
        }

        &self.buffer
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }
}

