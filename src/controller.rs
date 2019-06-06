
use super::message::{TimedMessage, Message};
use super::sequencer::Sequencer;
use super::cycle::Cycle;

pub struct Controller {
    pub sequencer: Sequencer,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            sequencer: Sequencer::new(),
        }
    }

    fn key_pressed(&mut self, message: jack::RawMidi, cycle: &Cycle, client: &jack::Client) {
        // Output in hex so we can compare to apc40 manual easily
        println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
        //println!("{}, {}, {}", message.bytes[0], message.bytes[1], message.bytes[2]);

        match message.bytes[1] {
            0x5B => client.transport_start(),
            0x5C => {
                let (state, _) = client.transport_query();
                match state {
                    1 => client.transport_stop(),
                    _ => client.transport_reposition(jack::Position::default()),
                };
            },
            _ => self.sequencer.key_pressed(message, cycle),
        }
    }

    fn key_released(&mut self, message: jack::RawMidi) {
        self.sequencer.key_released(message)
    }

    pub fn process_midi_messages<'a, I>(&mut self, input: I, cycle: &Cycle, client: &jack::Client) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                // Sysex events pass us a lot of data
                // It's cleaner to check the first byte though
                if message.bytes.len() > 3 {
                    self.process_sysex_message(message)
                } else {
                    self.process_message(message, cycle, client);
                    None
                }
            })
            .flatten()
            .collect()
    }

    fn process_sysex_message(&mut self, message: jack::RawMidi) -> Option<Vec<TimedMessage>> {
        // 0x06 = inquiry e, 0x02 = inquiry response
        // 0x47 = akai manufacturer, 0x73 = model nr
        if message.bytes[3] == 0x06 && message.bytes[4] == 0x02  
            && message.bytes[5] == 0x47 && message.bytes[6] == 0x73 
        {
            // Introduce ourselves to controller
            let message = Message::Introduction([0xF0, 0x47, message.bytes[13], 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
            let introduction = TimedMessage::new(0, message);

            // Rerender & draw what we want to see
            self.sequencer.reset();
            //self.sequencer.should_render = true;
            let render: Vec<TimedMessage> = self.sequencer.output_static_leds().into_iter()
                .map(|message| TimedMessage::new(128, message)).collect();

            let mut messages = vec![introduction];
            //messages.extend(clear);
            messages.extend(render);

            Some(messages)
        } else {
            None
        }
    }

    fn process_message(&mut self, message: jack::RawMidi, cycle: &Cycle, client: &jack::Client) {
        match message.bytes[0] {
            0x90...0x97 => self.key_pressed(message, cycle, client),
            0x80...0x87 => self.key_released(message),
            _ => (),
        }
    }
}

