
pub mod input;
mod lights;

use std::ops::Range;
use super::message::{TimedMessage, Message};
use super::cycle::ProcessCycle;
use super::phrase::*;
use super::sequencer::Sequencer;
use super::surface::*;
use super::port::MidiOut;
use super::mixer::*;
use super::TimebaseHandler;
use super::events::*;
use input::*;
use lights::*;

// Wait some cycles for sloooow apc's
const IDENTIFY_CYCLES: u8 = 3;

pub trait Controller {
    const CONTROLLER_ID: u8;
    const HEAD_COLOR: u8;
    const TAIL_COLOR: u8;

    fn ticks_in_grid(&self) -> u32;
    fn zoom_level(&self) -> u8;
    fn offset(&self, index: usize) -> u32;
    fn ticks_per_button(&self) -> u32 { self.ticks_in_grid() / 8 }
    fn button_to_ticks(&self, button: u8, offset: u32) -> u32 {
        button as u32 * self.ticks_per_button() + offset
    }

    // TODO - More grid logic
    fn grid(&mut self) -> &mut Grid;
    fn draw_tail(&mut self, mut x_range: Range<i32>, y: u8) {
        while let Some(x) = x_range.next() { self.grid().try_draw(x, y, Self::TAIL_COLOR) }
    }

    fn new(client: &jack::Client) -> Self;

    fn process_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer);
    fn output(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
}

// TODO - THis is not dry, seems like only way to clear this up at the moment is to create a nested "apc" struct,
// i don't really want to type "apc" after every access of self, so meh...
pub struct APC40 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    instrument_offset: u8,
    knob_offset: u8,

    patterns_shown: [u8; 16],
    zoom_level: u8,
    offsets: [u32; 16],
    base_notes: [u8; 16],

    cue_knob: CueKnob,

    grid: Grid,
    side: Side,
    instrument: WideRow,
    activator: WideRow,
    solo: WideRow,
    arm: WideRow,
}

impl APC40 {
    fn pattern_shown(&self, index: usize) -> u8 { self.patterns_shown[index] }
}

impl Controller for APC40 {
    const CONTROLLER_ID: u8 = 0;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT * 16 / self.zoom_level() as u32 }
    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn offset(&self, index: usize) -> u32 { self.offsets[index] }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC40 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC40 out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            // Offset the faders & sequence knobs by this value
            instrument_offset: 8,
            // Offset knobs by this value to support multiple groups
            knob_offset: 0,

            patterns_shown: [0; 16],
            zoom_level: 4,
            offsets: [0; 16],
            base_notes: [58; 16],

            cue_knob: CueKnob::new(),

            grid: Grid::new(),
            side: Side::new(),
            instrument: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            // TODO - Put length indicator here, get length from longest PatternEvent in phrases?
            arm: WideRow::new(0x30),
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
                    // TODO - Make sure every grid is re-initialized after identifying
                    self.identified_cycles = 1;

                    self.output.output_message(TimedMessage::new(0, message));
                },
                Event::KnobTurned { time, value, knob_type } => {
                    match knob_type {
                        KnobType::Cue => {
                            let delta_ticks = self.cue_knob.process_turn(value) as i32 * self.ticks_per_button() as i32;
        
                            // TODO - Move this offset stuff to struct as we will need it for

                            // Update offset of shown instrument when it's above
                            let new_offset = self.offset(surface.instrument_shown()) as i32 + delta_ticks;
                            if new_offset >= 0 {
                                self.offsets[surface.instrument_shown()] = new_offset as u32;
                            }
                        },
                        KnobType::Effect { time, index } => sequencer.knob_turned(time, index + self.knob_offset, value),
                    };
                },
                Event::FaderMoved { time, value, fader_type } => {
                    match fader_type {
                        FaderType::Track(index) => mixer.fader_adjusted(time, index + self.instrument_offset, value),
                        FaderType::Master => mixer.master_adjusted(time, value),
                    };
                },
                Event::ButtonPressed { time, button_type } => {
                    // Register press in memory to see if we double pressed
                    let is_double_pressed = surface.memory.press(Self::CONTROLLER_ID, cycle.time_at_frame(time), button_type);
                    // Get modifier (other currently pressed key)
                    let modifier = surface.memory.modifier(Self::CONTROLLER_ID, button_type);
                    let global_modifier = surface.memory.global_modifier(button_type);

                    match surface.view {
                        View::Instrument => {
                            let instrument = sequencer.get_instrument(surface.instrument_shown());
                        
                            match button_type {
                                ButtonType::Grid(x, y) => {
                                    // We subtract y from 4 as we want lower notes to be lower on
                                    // the grid, the grid counts from the top
                                    //let (start_tick, stop_tick) = self.grid_buttons_to_ticks(x, y, modifier);
                                    //let note = self.base_notes[surface.instrument_shown()] + (4 - y);
                                    //let offset = self.offset(surface.instrument_shown());

                                    //let pattern = instrument.get_pattern(self.pattern_shown(surface.instrument_shown()));
                                    //pattern.add_note_event(NoteEvent::on(start_tick + offset, note, 127));
                                    //pattern.add_note_event(NoteEvent::off(stop_tick + offset, note, 127));
                                },
                                ButtonType::Side(index) => {
                                    if is_double_pressed {
                                        instrument.get_pattern(index).switch_recording_state()
                                    } else {
                                        if let Some(ButtonType::Side(modifier_index)) = modifier {
                                            instrument.clone_pattern(modifier_index, index);
                                        } else if let Some(ButtonType::Shift) = global_modifier {
                                            instrument.get_pattern(index).clear_note_events();
                                        } else {
                                            self.patterns_shown[surface.instrument_shown()] = index; 
                                        }
                                    }
                                },
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    if zoom_level != 7 {
                                        self.zoom_level = zoom_level;
                                    }
                                },
                                ButtonType::Up => {
                                    let base_note = &mut self.base_notes[surface.instrument_shown()];
                                    let new_base_note = *base_note + 4;

                                    if new_base_note <= 118 { *base_note = new_base_note }
                                },
                                ButtonType::Down => {
                                    let base_note = &mut self.base_notes[surface.instrument_shown()];
                                    let new_base_note = *base_note - 4;

                                    if new_base_note >= 22 { *base_note = new_base_note }
                                },
                                ButtonType::Right => {
                                    let ticks_per_button = self.ticks_per_button();
                                    let offset = &mut self.offsets[surface.instrument_shown()];
                                    // There's 8 buttons, shift view one gridwidth to the right
                                    *offset = *offset + ticks_per_button * 8;
                                },
                                ButtonType::Left => {
                                    let ticks_per_button = self.ticks_per_button();
                                    let offset = &mut self.offsets[surface.instrument_shown()];
                                    let new_offset = *offset as i32 - (ticks_per_button * 8) as i32;

                                    *offset = if new_offset >= 0 { new_offset as u32 } else { 0 };
                                },
                                _ => (),
                            }
                        },
                        View::Sequence => {
                            let sequence = sequencer.get_sequence(surface.sequence_shown());

                            match button_type {
                                ButtonType::Grid(x, y) => {
                                    sequence.toggle_phrase(x + self.instrument_offset, y);
                                },
                                ButtonType::Side(index) => {
                                    sequence.toggle_row(index);
                                },
                                ButtonType::Activator(index) => {
                                    sequence.toggle_active(index + self.instrument_offset);
                                },
                                _ => (),
                            }
                        }
                    }

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
                        ButtonType::Instrument(index) => {
                            surface.toggle_instrument(index + self.instrument_offset);
                        },
                        ButtonType::Quantization => {
                            // TODO - Move quantizing & quantize_level to "keyboard"
                            sequencer.switch_quantizing();
                        },
                        _ => (),
                    }
                },
                Event::ButtonReleased { time, button_type } => {
                    surface.memory.release(Self::CONTROLLER_ID, cycle.time_at_frame(time), button_type);
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
        if self.identified_cycles == 0 {
            self.output.output_message(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        } else if self.identified_cycles < IDENTIFY_CYCLES {
            self.identified_cycles = self.identified_cycles + 1;
        } else {
            let instrument = sequencer.get_instrument(surface.instrument_shown());
            let pattern = instrument.get_pattern(self.pattern_shown(surface.instrument_shown()));

            // TODO Draw main grid

            self.side.draw(self.pattern_shown(surface.instrument_shown()), 1);
            if surface.instrument_shown() >= self.instrument_offset as usize {
                let instrument = surface.instrument_shown() - self.instrument_offset as usize;
                self.instrument.draw(instrument as u8, 1);
            }

            //for index in 0 .. self.indicator
            for index in 0 .. self.zoom_level { self.solo.draw(index, 1); }

            let mut output = vec![];
            output.append(&mut self.side.output());
            output.append(&mut self.instrument.output());
            output.append(&mut self.solo.output());

            for (channel, note, velocity) in output {
                self.output.output_message(TimedMessage::new(0, Message::Note([channel, note, 127])));
            }

        }

        self.output.write_midi(cycle.scope);
    }

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

    // TODO - Move this to "keyboard"
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
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,

    phrases_shown: [u8; 16],
    zoom_level: u8,
    offsets: [u32; 16],

    cue_knob: CueKnob,

    // Lights
    grid: Grid,
    side: Side,
    instrument: WideRow,
    activator: WideRow,
    solo: WideRow,
    arm: WideRow,
}

impl APC20 {
    fn phrase_shown(&self, index: usize) -> u8 { self.phrases_shown[index] }
}

impl Controller for APC20 {
    const CONTROLLER_ID: u8 = 1;
    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT * 4 * 16 / self.zoom_level() as u32 }
    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn offset(&self, index: usize) -> u32 { self.offsets[index] }

    fn grid(&mut self) -> &mut Grid { &mut self.grid }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC20 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC20 out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,

            phrases_shown: [0; 16],
            zoom_level: 4,
            offsets: [0; 16],

            cue_knob: CueKnob::new(),

            grid: Grid::new(),
            side: Side::new(),
            instrument: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            arm: WideRow::new(0x30),
        }
    }

    /*
     * Process input from controller jackport
     */
    fn process_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        for message in self.input.iter(cycle.scope) {
            let event = Event::new(message.time, message.bytes);
            let instrument = sequencer.get_instrument(surface.instrument_shown());
            let phrase = instrument.get_phrase(self.phrase_shown(surface.instrument_shown()));

            //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);
            // Only process channel note messages
            match event {
                Event::InquiryResponse(device_id) => {
                    // Introduce ourselves to controller
                    // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
                    // 0x42 is ableton alternate mode (all leds controlled from host)
                    let message = Message::Introduction([0xF0, 0x47, device_id, 0x7b, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                    // Make sure we stop inquiring
                    self.identified_cycles = 1;

                    self.output.output_message(TimedMessage::new(0, message));
                },
                Event::FaderMoved { time, value, fader_type } => {
                    // TODO - Pass these to "mixer"
                    match fader_type {
                        FaderType::Track(index) => mixer.fader_adjusted(time, index, value),
                        _ => (),
                    };
                },
                Event::KnobTurned { time, value, knob_type } => {
                    match knob_type {
                        KnobType::Cue => {
                            let max_offset = phrase.length() as i32 - self.ticks_in_grid() as i32;

                            if max_offset > 0 {
                                let delta_ticks = self.cue_knob.process_turn(value) as i32 * self.ticks_per_button() as i32;
                                // TODO - We shoudl readjust offset on zoom aswell
                                let new_offset = self.offset(surface.instrument_shown()) as i32 + delta_ticks;
                                let adjusted_offset = if new_offset < 0 { 0 } else if new_offset > max_offset { max_offset } else { new_offset };

                                self.offsets[surface.instrument_shown()] = adjusted_offset as u32;
                            }
                        },
                        _ => (),
                    }
                }
                Event::ButtonPressed { time, button_type } => {
                    // Register press in memory to see if we double pressed
                    let is_double_pressed = surface.memory.press(Self::CONTROLLER_ID, cycle.time_at_frame(time), button_type);
                    // Get modifier (other currently pressed key)
                    let modifier = surface.memory.modifier(Self::CONTROLLER_ID, button_type);
                    let global_modifier = surface.memory.global_modifier(button_type);

                    match surface.view {
                        View::Instrument => {
                            match button_type {
                                ButtonType::Grid(x, pattern) => {
                                    let offset = self.offset(surface.instrument_shown());

                                    let mut start_tick = self.button_to_ticks(x, offset);
                                    let stop_tick = self.button_to_ticks(x + 1, offset);

                                    // Should we delete the pattern we're clicking?
                                    if let (None, true) = (modifier, phrase.contains_events_starting_between(start_tick, stop_tick, pattern)) {
                                        phrase.remove_events_starting_between(start_tick, stop_tick, pattern);
                                    } else {
                                        // Add pattern get x from modifier when its a grid button in the same row
                                        if let Some(ButtonType::Grid(mod_x, mod_pattern)) = modifier {
                                            if mod_pattern == pattern { 
                                                start_tick = self.button_to_ticks(mod_x, offset);
                                            }
                                        }

                                        phrase.try_add_starting_event(PatternEvent::new(start_tick, None, pattern));
                                        let mut event = phrase.get_last_event_on_row(pattern);
                                        event.set_stop(stop_tick);

                                        phrase.add_complete_event(event);
                                    }

                                },
                                ButtonType::Side(index) => {
                                    if let Some(ButtonType::Side(modifier_index)) = modifier {
                                        instrument.clone_phrase(modifier_index, index);
                                    } else if let Some(ButtonType::Shift) = global_modifier {
                                        instrument.get_phrase(index).clear_events();
                                    } else {
                                        self.phrases_shown[surface.instrument_shown()] = index;
                                    }
                                },
                                ButtonType::Activator(index) => {
                                    phrase.set_length(Phrase::default_length() * (index as u32 + 1));
                                },
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    if zoom_level != 7 {
                                        self.zoom_level = zoom_level;
                                    }
                                },
                                _ => (),
                            }
                        },
                        View::Sequence => {
                            let sequence = sequencer.get_sequence(surface.sequence_shown());
                        
                            match button_type {
                                ButtonType::Grid(x, y) => sequence.toggle_phrase(x, y),
                                // TODO - Only switch rows on APC20
                                ButtonType::Side(index) => sequence.toggle_row(index),
                                ButtonType::Activator(index) => sequence.toggle_active(index),
                                _ => (),
                            }
                        }
                    }

                    match button_type {
                        ButtonType::Instrument(index) => surface.toggle_instrument(index),
                        _ => (),
                    }
                },
                Event::ButtonReleased { time, button_type } => {
                    surface.memory.release(Self::CONTROLLER_ID, cycle.time_at_frame(time), button_type);
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
        if self.identified_cycles == 0 {
            self.output.output_message(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        } else if self.identified_cycles < IDENTIFY_CYCLES {
            self.identified_cycles = self.identified_cycles + 1;
        } else {
            let instrument = sequencer.get_instrument(surface.instrument_shown());
            let phrase = instrument.get_phrase(self.phrase_shown(surface.instrument_shown()));
            let offset = self.offset(surface.instrument_shown());
            let grid_stop = offset + self.ticks_in_grid();

            // Draw main grid
            phrase.pattern_events.iter()
                .filter(|event| { 
                    let grid_contains_event = event.start < grid_stop 
                        && (event.stop.is_none() || event.stop.unwrap() > offset);

                    grid_contains_event || event.is_looping()
                })
                .for_each(|event| {
                    let button_ticks = self.ticks_per_button() as i32;

                    // Get buttons from event ticks
                    let max_button = phrase.length() as i32 / button_ticks;
                    let start_button = (event.start as i32 - offset as i32) / button_ticks;
                    let stop_button = if event.stop.is_none() { 
                        start_button + 1
                    } else { 
                        (event.stop.unwrap() as i32 - offset as i32) / button_ticks
                    };

                    // Always draw first button head
                    self.grid.try_draw(start_button, event.pattern, 3);
                    // Draw tail depending on wether this is looping note
                    if stop_button > start_button {
                        self.draw_tail((start_button + 1) .. stop_button, event.pattern);
                    } else {
                        self.draw_tail((start_button + 1) .. max_button, event.pattern);
                        self.draw_tail(0 .. stop_button, event.pattern);
                    }
                });

            self.side.draw(self.phrase_shown(surface.instrument_shown()), 1);
            self.instrument.draw(surface.instrument_shown() as u8, 1);

            //for index in 0 .. self.indicator
            for index in 0 .. (phrase.length() / Phrase::default_length()) {
                self.activator.draw(index as u8, 1);
            }
            for index in 0 .. self.zoom_level { self.solo.draw(index, 1); }

            let mut output = vec![];
            output.append(&mut self.grid.output());
            output.append(&mut self.side.output());
            output.append(&mut self.instrument.output());
            output.append(&mut self.activator.output());
            output.append(&mut self.solo.output());

            for (channel, note, velocity) in output {
                self.output.output_message(TimedMessage::new(0, Message::Note([channel, note, velocity])));
            }
        }

        // TODO - We probably don't have to cache messages in vec anymore, as they only originate
        // from this function
        self.output.write_midi(cycle.scope);
    }
}
