
mod input;

use super::message::{TimedMessage, Message};
use super::cycle::ProcessCycle;
use super::sequencer::Sequencer;
use super::surface::*;
use super::port::MidiOut;
use super::mixer::*;
use input::*;

pub trait Controller {
    fn new(client: &jack::Client) -> Self;

    fn process_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer);
    fn output(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
}

// TODO - THis is not dry, seems like only way to clear this up at the moment is to create a nested "apc" struct,
// i don't really want to type "apc" after every access of self, so meh...
pub struct APC40 {
    memory: Memory,

    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    is_identified: bool,
    offset: u8,
}

impl Controller for APC40 {
    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC40 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC40 out", jack::MidiOut::default()).unwrap();
        
        Self {
            memory: Memory::new(),

            input,
            output: MidiOut::new(output),

            is_identified: false,
            // Offset the faders & sequence knobs by this value
            offset: 8,
        }
    }

    /*
     * Process input from controller jackport
     */
    fn process_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        for message in self.input.iter(cycle.scope) {
            let event = Event::new(message.time, message.bytes);

            //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
            // Only process channel note messages
            match event {
                Event::InquiryResponse(device_id) => {
                    // Introduce ourselves to controller
                    // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
                    // 0x42 is ableton alternate mode (all leds controlled from host)
                    let message = Message::Introduction([0xF0, 0x47, device_id, 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                    // Make sure we stop inquiring
                    self.is_identified = true;

                    self.output.output_message(TimedMessage::new(0, message));
                },
                Event::KnobTurned { value, knob_type } => {
                    match knob_type {
                        KnobType::Cue => sequencer.cue_knob_turned(value),
                        KnobType::Effect { time, index } => sequencer.knob_turned(time, index, value),
                    };
                },
                Event::FaderMoved { time, value, fader_type } => {
                    // TODO - Pass these to "mixer"
                    match fader_type {
                        FaderType::Track(index) => mixer.fader_adjusted(time, index + self.offset, value),
                        FaderType::Master => mixer.master_adjusted(time, value),
                    };
                },
                Event::ButtonPressed { time, button_type } => {
                    // First get modifier (other currently pressed key), before registering current press in memory
                    let modifier = self.memory.modifier();
                    // Register press in memory to see if we double pressed
                    let is_double_pressed = self.memory.press(cycle.time_at_frame(time), button_type);

                    match button_type {
                        ButtonType::Play => cycle.client.transport_start(),
                        ButtonType::Stop => {
                            // Reset to 0 when we press stop button but we're already stopped
                            let (state, _) = cycle.client.transport_query();
                            match state {
                                1 => cycle.client.transport_stop(),
                                _ => cycle.client.transport_reposition(jack::Position::default()),
                            };
                        },
                        ButtonType::Sequence(index) => {
                            if let Some(ButtonType::Shift) = modifier {
                                sequencer.queue_sequence(index);
                            } else {
                                surface.toggle_sequence(index);
                            }
                        },
                        ButtonType::Grid { x, y } => {
                            match surface.view {
                                View::Instrument => {
                                
                                },
                                View::Sequence => {
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_phrase(x + self.offset, y);
                                }
                            }
                        },
                        ButtonType::Playable(index) => {
                            match surface.view {
                                View::Instrument => {
                                    let instrument = sequencer.get_instrument(surface.instrument_shown());

                                    if is_double_pressed {
                                        instrument.get_pattern(index).switch_recording_state()
                                    } else {
                                        if let Some(ButtonType::Playable(modifier_index)) = modifier {
                                            instrument.clone_pattern(modifier_index, index);
                                        } else {
                                            surface.show_pattern(index);
                                        }
                                    }
                                },
                                View::Sequence => {
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_row(index);
                                }
                            }
                        },
                        ButtonType::Instrument(index) => {
                            surface.toggle_instrument(index + self.offset);
                        },
                        ButtonType::Quantization => {
                            sequencer.switch_quantizing();
                        },
                        ButtonType::Activator(index) => {
                            match surface.view {
                                View::Instrument => {
                                     // TODO - Select knob group
                                    //0x3A ..= 0x3D => self.switch_knob_group(message.bytes[1] - 0x3A),
                                },
                                View::Sequence => {
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_active(index + self.offset);
                                }
                            }
                        }
                        _ => {
                            // Always single press ?
                            //sequencer.key_pressed(message);
                        },
                    }
                },
                Event::ButtonReleased { time, button_type } => {
                    self.memory.release(cycle.time_at_frame(time), button_type);
                },
                _ => (),
            }
        }
    }

    /*
     * Output to jack
     */
    fn output(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) {
        // Identify when no controller found yet
        if ! self.is_identified {
            self.output.clear_output_buffer();
            self.output.output_message(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        }

        self.output.write_midi(cycle.scope);
    }

    /*
    // Process messages from APC controller keys being pushed
    pub fn process_sysex_input<'a, I>(&mut self, input: I, cycle: &Cycle, client: &jack::Client) -> Vec<TimedMessage>
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .filter_map(|message| {
                //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
                // 0x06 = inquiry e, 0x02 = inquiry response
                // 0x47 = akai manufacturer, 0x73 = model nr
                if message.bytes[0] == 0xF0 &&
                    message.bytes[3] == 0x06 && message.bytes[4] == 0x02  
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
                    messages.extend(self.sequencer.output_static(true));

                    Some(messages)
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    // Process messages from APC controller keys being pushed
    pub fn process_apc_note_messages<'a, I>(&mut self, input: I, cycle: &Cycle, client: &jack::Client)
        where
            I: Iterator<Item = jack::RawMidi<'a>>,
    {
        input
            .for_each(|message| {
                //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
                // Only process channel note messages
                match message.bytes[0] {
                    0xB0 => {
                        if message.bytes[1] == 0x2F {
                            self.sequencer.cue_knob_turned(message.bytes[2]);
                        }
                    },
                    0x90 ..= 0x9F => {
                        let pressed_key = PressedKey { 
                            time: cycle.absolute_start + message.time, 
                            channel: message.bytes[0],
                            key: message.bytes[1],
                        };

                        // Remove keypresses that are not within double press range
                        self.pressed_keys.retain(|previous| {
                            pressed_key.time - previous.time < Controller::DOUBLE_PRESS_TICKS
                        });

                        // Check for old keypresses matching currently pressed key
                        let double_presses: Vec<bool> = self.pressed_keys.iter()
                            .filter_map(|previous| {
                                if previous.channel == pressed_key.channel && previous.key == pressed_key.key {
                                    Some(true)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        // Always single press 
                        match message.bytes[1] {
                            0x5B => { client.transport_start() },
                            0x5C => {
                                let (state, _) = client.transport_query();
                                match state {
                                    1 => client.transport_stop(),
                                    _ => client.transport_reposition(jack::Position::default()),
                                };
                            },
                            _ => self.sequencer.key_pressed(message),
                        }

                        // Double pressed_key when its there
                        if double_presses.len() > 0 {
                            self.sequencer.key_double_pressed(message);
                        }

                        // Save pressed_key
                        self.pressed_keys.push(pressed_key);

                    },
                    0x80 ..= 0x8F => self.sequencer.key_released(message),
                    _ => (),
                }
            })
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
                    0xB0 ..= 0xB8 => {
                        match message.bytes[1] {
                            // APC knobs are ordered weird, reorder them from to 0..16
                            0x10..=0x17 => Some(self.sequencer.knob_turned(message.time, message.bytes[1] - 8, message.bytes[2])),
                            0x30..=0x37 => Some(self.sequencer.knob_turned(message.time, message.bytes[1] - 48, message.bytes[2])),
                            0x7 => Some(self.sequencer.fader_adjusted(message.time, message.bytes[0] - 0xB0, message.bytes[2])),
                            0xE => Some(self.sequencer.master_adjusted(message.time, message.bytes[2])),
                            _ => None,
                        }
                    },
                    _ => None,
                }
            })
            .flatten()
            .collect()
    }
    */

        /*
    // Process incoming control change messages from plugins of which parameters were changed
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
                let option = match message.bytes[0] {
                    0x90 | 0x80 => Some((self.sequencer.keyboard_target, 0)),
                    0x99 | 0x89 => Some((self.sequencer.drumpad_target, 9)),
                    _ => None,
                };

                // Only process channel note messages
                if let Some((index, offset)) = option {
                    Some(self.sequencer.recording_key_played(index + self.sequencer.instrument_group * 8, message.bytes[0] - offset, cycle, message))
                } else {
                    None
                }
            })
            .collect()
    }
    */
}


pub struct APC20 {
    memory: Memory,

    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    is_identified: bool,
}

impl Controller for APC20 {
    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC20 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC20 out", jack::MidiOut::default()).unwrap();
        
        Self {
            memory: Memory::new(),

            input,
            output: MidiOut::new(output),

            is_identified: false,
        }
    }

    /*
     * Process input from controller jackport
     */
    fn process_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        for message in self.input.iter(cycle.scope) {
            let event = Event::new(message.time, message.bytes);

            //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
            // Only process channel note messages
            match event {
                Event::InquiryResponse(device_id) => {
                    // Introduce ourselves to controller
                    // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
                    // 0x42 is ableton alternate mode (all leds controlled from host)
                    let message = Message::Introduction([0xF0, 0x47, device_id, 0x7b, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                    // Make sure we stop inquiring
                    self.is_identified = true;

                    self.output.output_message(TimedMessage::new(0, message));
                },
                Event::FaderMoved { time, value, fader_type } => {
                    // TODO - Pass these to "mixer"
                    match fader_type {
                        FaderType::Track(index) => mixer.fader_adjusted(time, index, value),
                        _ => (),
                    };
                },
                Event::ButtonPressed { time, button_type } => {
                    // First get modifier (other currently pressed key), before registering current press in memory
                    let modifier = self.memory.modifier();
                    // Register press in memory to see if we double pressed
                    let is_double_pressed = self.memory.press(cycle.time_at_frame(time), button_type);

                    match button_type {
                        ButtonType::Grid { x, y } => {
                            match surface.view {
                                View::Instrument => {
                                
                                },
                                View::Sequence => {
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_phrase(x, y);
                                }
                            }
                        },
                        ButtonType::Playable(index) => {
                            match surface.view {
                                View::Instrument => {
                                    let instrument = sequencer.get_instrument(surface.instrument_shown());

                                    if let Some(ButtonType::Playable(modifier_index)) = modifier {
                                        instrument.clone_phrase(modifier_index, index);
                                    } else {
                                        surface.show_phrase(index);
                                    }
                                },
                                View::Sequence => {
                                    // TODO - Only switch rows on APC20
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_row(index);
                                }
                            }
                        },
                        ButtonType::Instrument(index) => {
                            surface.toggle_instrument(index);
                        },
                        ButtonType::Activator(index) => {
                            match surface.view {
                                View::Instrument => {
                                     // TODO - Select knob group
                                    //0x3A ..= 0x3D => self.switch_knob_group(message.bytes[1] - 0x3A),
                                },
                                View::Sequence => {
                                    sequencer.get_sequence(surface.sequence_shown()).toggle_active(index);
                                }
                            }
                        }
                        _ => {
                            // Always single press ?
                            //sequencer.key_pressed(message);
                        },
                    }
                },
                Event::ButtonReleased { time, button_type } => {
                    self.memory.release(cycle.time_at_frame(time), button_type);
                },
                _ => (),
            }
        }
    }

    /*
     * Output to jack
     */
    fn output(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) {
        // Identify when no controller found yet
        if ! self.is_identified {
            self.output.clear_output_buffer();
            self.output.output_message(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        }

        self.output.write_midi(cycle.scope);
    }
}
