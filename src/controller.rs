
use super::message::{TimedMessage, Message};
use super::sequencer::Sequencer;
use super::cycle::Cycle;
use super::handlers::TimebaseHandler;

pub struct Controller {
    pub sequencer: Sequencer,
    key_presses: Vec<KeyPress>,
}

#[derive(Debug)]
struct KeyPress {
    time: u32,
    channel: u8,
    key: u8,
}

impl Controller {
    const DOUBLE_PRESS_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT / 2;

    pub fn new() -> Self {
        Controller {
            sequencer: Sequencer::new(),
            key_presses: vec![],
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
    pub fn process_apc_note_messages<'a, I>(&mut self, input: I, cycle: &Cycle, client: &jack::Client) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
                // Only process channel note messages
                match message.bytes[0] {
                    0xF0 => self.process_sysex_message(message),
                    0x90..=0x9F => {
                        let keypress = KeyPress { 
                            time: cycle.absolute_start + message.time, 
                            channel: message.bytes[0],
                            key: message.bytes[1],
                        };

                        // Remove keypresses that are not within double press range
                        self.key_presses.retain(|previous| {
                            keypress.time - previous.time < Controller::DOUBLE_PRESS_TICKS
                        });

                        // Check for old keypresses matching currently pressed key
                        let double_presses: Vec<bool> = self.key_presses.iter()
                            .filter_map(|previous| {
                                if previous.channel == keypress.channel && previous.key == keypress.key {
                                    Some(true)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let mut output: Vec<TimedMessage> = vec![];

                        // Always single press 
                        if let Some(messages) = self.key_pressed(message, client) {
                            output.extend(messages);
                        }

                        // Double keypress when its there
                        if double_presses.len() > 0 {
                            if let Some(messages) = self.sequencer.key_double_pressed(message) {
                                output.extend(messages);
                            }
                        }

                        // Save keypress
                        self.key_presses.push(keypress);

                        Some(output)
                    },
                    0x80..=0x8F => self.sequencer.key_released(message),
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
                    0xB0..=0xBF => self.sequencer.plugin_parameter_changed(message),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn process_instrument_messages<'a, I>(&mut self, cycle: &Cycle, input: I) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                // Only process channel note messages
                match message.bytes[0] {
                    0x90 | 0x80 => Some(self.sequencer.recording_key_played(self.sequencer.keyboard_target, 0, cycle, message)),
                    0x99 | 0x89 => Some(self.sequencer.recording_key_played(self.sequencer.drumpad_target, 9, cycle, message)),
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

