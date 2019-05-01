
use super::message::{TimedMessage, Message};
use super::sequencer::Sequencer;

pub struct Controller {
    pub sequencer: Sequencer,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            sequencer: Sequencer::new(),
        }
    }

    fn key_pressed(&mut self, message: jack::RawMidi, client: &jack::Client) -> Option<Vec<Message>> {
        // Output in hex so we can compare to apc40 manual easily
        println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
        //println!("{}, {}, {}", message.bytes[0], message.bytes[1], message.bytes[2]);

        match message.bytes[1] {
            0x5B => { 
                client.transport_start(); 
                None 
            },
            0x5C => {
                let (state, _) = client.transport_query();
                match state {
                    1 => client.transport_stop(),
                    _ => {
                        let pos = jack::Position::default();
                        client.transport_reposition(pos);
                    }
                };
                None
            },
            _ => Some(self.sequencer.key_pressed(message)),
        }
    }

    fn key_released(&mut self, _event: jack::RawMidi, _client: &jack::Client) -> Option<Vec<Message>> {
        None
    }

    pub fn process_midi_messages<'a, I>(&mut self, messages: I, client: &jack::Client) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        messages
            .filter_map(|message| {
                // Sysex events pass us a lot of data
                // It's cleaner to check the first byte though
                if message.bytes.len() > 3 {
                    self.process_sysex_message(message)
                } else {
                    self.process_message(message, client).and_then(|messages| {
                        Some(messages.into_iter().map(|message| { TimedMessage::new(0, message) }).collect())
                    })
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
            let introduction = Message::Introduction([0xF0, 0x47, message.bytes[13], 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);

            let mut timed_messages = vec![TimedMessage::new(0, introduction)];
            let sequencer_messages: Vec<TimedMessage> = vec![ self.sequencer.clear(true), self.sequencer.draw() ]
                .into_iter()
                .flatten()
                .map(|message| { TimedMessage::new(64, message) })
                .collect();

            timed_messages.extend(sequencer_messages);
            Some(timed_messages)
        } else {
            None
        }
    }

    fn process_message(&mut self, message: jack::RawMidi, client: &jack::Client) -> Option<Vec<Message>> {
        match message.bytes[0] {
            0x90...0x97 => self.key_pressed(message, client),
            0x80...0x87 => self.key_released(message, client),
            _ => None,
        }
    }
}

