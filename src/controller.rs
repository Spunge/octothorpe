
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

    fn key_pressed(&mut self, message: jack::RawMidi, client: &jack::Client) -> Option<Vec<TimedMessage>> {
        // Output in hex so we can compare to apc40 manual easily
        //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
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
                    _ => client.transport_reposition(jack::Position::default()),
                };
                None
            },
            _ => self.sequencer.key_pressed(message),
        }
    }

    // Process messages from APC controller keys being pushed
    pub fn process_apc_note_messages<'a, I>(&mut self, input: I, client: &jack::Client) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
                // Only process channel note messages
                match message.bytes[0] {
                    0xF0 => self.process_sysex_message(message),
                    0x90...0x9F => self.key_pressed(message, client),
                    0x80...0x8F => self.sequencer.key_released(message),
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }

    // Process messages from APC controller keys being pushed
    pub fn process_apc_control_change_messages<'a, I>(&mut self, input: I) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
                // Only process channel note messages
                match message.bytes[0] {
                    0xB0 => self.sequencer.control_changed(message),
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }

    // Process messages from APC controller knobs being turned
    // This is a seperate function as we want to send responses to these messages to a diferent port
    pub fn process_plugin_control_change_messages<'a, I>(&mut self, input: I) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                // Only process channel note messages
                match message.bytes[0] {
                    0xB0...0xBF => self.sequencer.plugin_parameter_changed(message),
                    _ => None,
                }
            })
            .collect()
    }

    fn process_sysex_message(&mut self, message: jack::RawMidi) -> Option<Vec<TimedMessage>> {
        // 0x06 = inquiry e, 0x02 = inquiry response
        // 0x47 = akai manufacturer, 0x73 = model nr
        if message.bytes[3] == 0x06 && message.bytes[4] == 0x02  
            && message.bytes[5] == 0x47 && message.bytes[6] == 0x73 
        {
            // Introduce ourselves to controller
            // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
            // 0x42 is ableton alternate mode (all leds controlled from host)
            let message = Message::Introduction([0xF0, 0x47, message.bytes[13], 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
            let introduction = TimedMessage::new(0, message);

            // Rerender & draw what we want to see
            self.sequencer.reset();
            let mut messages = vec![introduction];
            // TODO - Before we timed the messages after introduction to 128 frames, why?
            messages.extend(self.sequencer.output_static());

            Some(messages)
        } else {
            None
        }
    }
}

