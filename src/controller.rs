

// Get string as letter vector
fn get_text_as_vector(text: String) -> Vec<u8> {
    // Get letters array from string

    (0..5)
        .map(|row| {
            let letters = text.chars();

            letters
                .map(|letter| { 
                    // Get letter vector
                    let vec = get_letter(letter);
                    // Get width of letter
                    let width = vec.len() / 5;

                    // Return slice of it based on row
                    let mut sliced = vec[std::ops::Range { start: row * width, end: (row + 1) * width }].to_vec();
                    // Add whitespace
                    sliced.push(0);
                    // Return slice
                    sliced
                })
                // Fold into one vector
                .fold(Vec::new(), |mut acc, mut x| { acc.append(&mut x); acc })
        })
        // Fold into one vector
        .fold(Vec::new(), |mut acc, mut x| { acc.append(&mut x); acc })
}

fn get_letter(letter: char) -> Vec<u8> {
    match letter {
        'h' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
        'a' => vec![0, 1, 0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
        'c' => vec![1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1],
        'k' => vec![1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1],
        'e' => vec![1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1],
        'd' => vec![1, 1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0],
        'b' => vec![1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 0],
        'y' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0],
        'r' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
        'o' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
        't' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
        _ => vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    }
}

#[derive(Debug)]
pub struct Controller {
    buffer: Vec<super::Message>,
    is_identified: bool,
    device_id: u8,

    letters: Vec<u8>,

    tick_counter: u32,
    ticks_per_frame: u32,
}

impl<'a> Controller {
    pub fn new() -> Self {
        println!("{:?}", get_text_as_vector("hacked by root".to_string()));

        Controller {
            is_identified: false,
            device_id: 0,
            buffer: Vec::new(),

            letters: get_text_as_vector("hacked by root".to_string()),

            tick_counter: 0,
            ticks_per_frame: 1000,
        }
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
                        super::RawMessage::Note([0x90 + x, 0x35 + y, 0x05]),
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
        self.tick_counter += 1;

        if ! self.is_identified {
            self.inquire();
        }

        self.print_frame();

        &self.buffer
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    fn print_frame(&mut self) {
        // Loop through x coords
        for x in 0..8 {
            for y in 0..5 {
                let value = self.letters[x + self.letters.len() / 5 * y];

                self.buffer.push(super::Message::new(
                    0,
                    super::RawMessage::Note([0x90 + x as u8, 0x35 + y as u8, value]),
                ));
            }
        }
    }

    fn clear_grid(&mut self) {
        for x in 0..8 {
            for y in 0..5 {
                self.buffer.push(super::Message::new(
                    0,
                    super::RawMessage::Note([0x90 + x, 0x35 + y, 0x00]),
                ));
            }
        }
    }
}

