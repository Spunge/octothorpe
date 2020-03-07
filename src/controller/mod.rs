
pub mod input;
mod lights;

use std::ops::Range;
use super::TickRange;
use super::message::{TimedMessage, Message};
use super::cycle::ProcessCycle;
use super::loopable::*;
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
const LENGTH_INDICATOR_USECS: u64 = 600000;

pub trait APC {
    type Loopable: Loopable;

    const CONTROLLER_ID: u8;
    const INSTRUMENT_OFFSET: u8;
    const HEAD_COLOR: u8;
    const TAIL_COLOR: u8;

    fn identified_cycles(&self) -> u8;
    fn set_identified_cycles(&mut self, cycles: u8);

    fn zoom_level(&self) -> u8;
    fn set_zoom_level(&mut self, level: u8);
    fn ticks_in_grid(&self) -> u32;
    fn ticks_per_button(&self) -> u32 { self.ticks_in_grid() / 8 }
    fn button_to_ticks(&self, button: u8, offset: u32) -> u32 {
        button as u32 * self.ticks_per_button() + offset
    }

    fn offset(&self, index: usize) -> u32;
    fn set_offset(&mut self, index: usize, ticks: u32);

    fn max_offset(&self, length: u32) -> u32 {
        let max = length as i32 - self.ticks_in_grid() as i32;
        if max > 0 { max as u32 } else { 0 }
    }
    fn adjusted_offset(&self, instrument: usize, max_offset: u32, delta_buttons: i8) -> u32 {
        let delta_ticks = delta_buttons as i32 * self.ticks_per_button() as i32;
        let new_offset = self.offset(instrument) as i32 + delta_ticks;

        if new_offset >= 0 {
            if new_offset < max_offset as i32 { new_offset as u32 } else { max_offset }
        } else { 0 }
    }

    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable;

    fn cue_knob(&mut self) -> &mut CueKnob;
    fn grid(&mut self) -> &mut Grid;
    fn instrument(&mut self) -> &mut WideRow;
    fn indicator(&mut self) -> &mut WideRow;
    fn solo(&mut self) -> &mut WideRow;

    /*
     * Remove existing events when there's starting events in tick range, otherwise, remove tick
     * range so we can add new event
     */
    fn should_add_event(&self, loopable: &mut impl Loopable, modifier: Option<ButtonType>, x: u8, y: u8, offset: u32, row: u8) -> Option<TickRange> {
        let mut tick_range = TickRange::new(self.button_to_ticks(x, offset), self.button_to_ticks(x + 1, offset));

        // Should we delete the event we're clicking?
        if let (None, true) = (modifier, loopable.contains_events_starting_in(tick_range, row)) {
            loopable.remove_events_starting_in(tick_range, row);
            None
        } else {
            // Add event get x from modifier when its a grid button in the same row
            if let Some(ButtonType::Grid(mod_x, mod_y)) = modifier {
                if mod_y == y { 
                    tick_range.start = self.button_to_ticks(mod_x, offset);
                }
            }

            Some(tick_range)
        }
    }

    // TODO - only draw length indicator at position 0 only when we are precisely at 0
    fn output_indicator<F>(&mut self, cycle: &ProcessCycle, filters: &[F], surface: &Surface, length: u32) -> Vec<TimedMessage> where F: Fn(&InputEventType) -> bool {
        let usecs = cycle.time_stop - LENGTH_INDICATOR_USECS;
        let mut frame = 0;

        // TODO - move this timing logic to seperate function when we need it for other things
        // Do we need to draw length indicator, and when?
        if let Some(usecs) = surface.event_memory.last_occurred_event_after(Self::CONTROLLER_ID, filters, usecs) {
            let usecs_ago = cycle.time_stop - usecs;
            let hide_in_usecs = LENGTH_INDICATOR_USECS - usecs_ago;

            if hide_in_usecs < cycle.usecs() {
                frame = hide_in_usecs as u32 * cycle.scope.n_frames() / cycle.usecs() as u32;
            } else {
                let length_buttons = (self.indicator().width() as u32 * self.ticks_in_grid() / length) as u8;
                //let length_buttons = self.indicator().width() / (length / self.ticks_in_grid()) as u8;
                let offset_buttons = (self.offset(surface.instrument_shown()) / self.ticks_per_button()) as u8;
                let start_button = offset_buttons * length_buttons / self.indicator().width();
                let stop_button = start_button + length_buttons;
                for index in start_button .. stop_button {
                    self.indicator().draw(index as u8, 1);
                }
            }
        }

        self.indicator().output_messages(frame)
    }

    /*
     * Draw note or pattern events into main grid of controller
     */
    fn draw_events<'a>(&mut self, events: impl Iterator<Item = &'a (impl LoopableEvent + 'a)>, offset_x: u32, offset_y: u8) {
        let grid_stop = offset_x + self.ticks_in_grid();

        // Draw main grid
        events
            .filter(|event| { 
                let grid_contains_event = event.start() < grid_stop 
                    && (event.stop().is_none() || event.stop().unwrap() > offset_x);

                grid_contains_event || event.is_looping()
            })
            .for_each(|event| {
                let button_ticks = self.ticks_per_button() as i32;

                // Get buttons from event ticks
                let max_button = self.grid().width() as i32;
                let start_button = (event.start() as i32 - offset_x as i32) / button_ticks;
                let stop_button = if event.stop().is_none() { 
                    start_button + 1
                } else { 
                    // Could be event is to short for 1 button, in that case, draw 1 button
                    let button = (event.stop().unwrap() as i32 - offset_x as i32) / button_ticks;
                    if button <= start_button { start_button + 1 } else { button }
                };

                // Flip grid around to show higher notes higher on the grid (for patterns this does not matter)
                let row = 4 - event.row(offset_y);

                // Always draw first button head
                self.grid().try_draw(start_button, row, Self::HEAD_COLOR);
                // Draw tail depending on wether this is looping note
                if stop_button > start_button {
                    self.draw_tail((start_button + 1) .. stop_button, row);
                } else {
                    self.draw_tail((start_button + 1) .. max_button, row);
                    self.draw_tail(0 .. stop_button, row);
                }
            });
    }

    fn draw_tail(&mut self, mut x_range: Range<i32>, y: u8) {
        while let Some(x) = x_range.next() { self.grid().try_draw(x, y, Self::TAIL_COLOR) }
    }

    fn new(client: &jack::Client) -> Self;

    /*
     * Process incoming midi, handle generic midi here, pass controller specific input to
     * controller via process_inputevent
     */ 
    fn process_midi_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        for event in self.input_events(cycle.scope) {
            // Only process channel note messages
            match event.event_type {
                InputEventType::InquiryResponse(local_id, device_id) => {
                    // Introduce ourselves to controller
                    // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
                    // 0x42 is ableton alternate mode (all leds controlled from host)
                    let message = Message::Introduction([0xF0, 0x47, local_id, device_id, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                    // Make sure we stop inquiring
                    // TODO - Make sure every grid is re-initialized after identifying
                    self.set_identified_cycles(1);

                    self.output().output_message(TimedMessage::new(0, message));
                },
                InputEventType::FaderMoved { value, fader_type: FaderType::Track(index) } => {
                    mixer.fader_adjusted(event.time, index + Self::INSTRUMENT_OFFSET, value);
                },
                // TODO - Shift events in loopable to right/left when holding shift
                InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                    let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                    let is_first_turn = surface.event_memory
                        .last_occurred_event_after(Self::CONTROLLER_ID, &[InputEvent::is_cue_knob], usecs)
                        .is_none();

                    let delta_buttons = self.cue_knob().process_turn(value, is_first_turn);
                    let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                    let offset = self.adjusted_offset(surface.instrument_shown(), max_offset, delta_buttons);
                    self.set_offset(surface.instrument_shown(), offset);
                },
                InputEventType::ButtonPressed(button_type) => {
                    // Register press in memory to keep track of modifing buttons
                    surface.button_memory.press(Self::CONTROLLER_ID, button_type);

                    // Do the right thing in the right visualization
                    match surface.view {
                        View::Instrument => {
                            match button_type {
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    if zoom_level != 7 {
                                        self.set_zoom_level(zoom_level);

                                        // It could happen that we're moved out of range when zooming out
                                        let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                                        if self.offset(surface.instrument_shown()) > max_offset {
                                            self.set_offset(surface.instrument_shown(), max_offset);
                                        }
                                    }
                                },
                                _ => (),
                            }
                        },
                        View::Sequence => {
                            let sequence = sequencer.get_sequence(surface.sequence_shown());

                            match button_type {
                                ButtonType::Grid(x, phrase) => {
                                    let instrument = (x + Self::INSTRUMENT_OFFSET) as usize;
                                    if let None = sequence.get_phrase(instrument) {
                                        sequence.set_phrase(instrument, phrase);
                                    } else {
                                        sequence.unset_phrase(instrument)
                                    }
                                },
                                ButtonType::Side(phrase) => sequence.set_phrases(phrase),
                                ButtonType::Activator(instrument) => {
                                    sequence.toggle_active((instrument + Self::INSTRUMENT_OFFSET) as usize)
                                },
                                _ => (),
                            }
                        }
                    }

                    // Independent of current view
                    match button_type {
                        ButtonType::Instrument(index) => surface.toggle_instrument(index + Self::INSTRUMENT_OFFSET),
                        ButtonType::Master => surface.switch_view(),
                        _ => self.process_inputevent(&event, cycle, sequencer, surface, mixer),
                    }
                },
                InputEventType::ButtonReleased(button_type) => {
                    surface.button_memory.release(Self::CONTROLLER_ID, cycle.time_at_frame(event.time), button_type);
                },
                // This message is controller specific, handle it accordingly
                _ => self.process_inputevent(&event, cycle, sequencer, surface, mixer),
            }

            // Keep track of event so we can use it to calculate double presses etc.
            surface.event_memory.register_event(Self::CONTROLLER_ID, cycle.time_at_frame(event.time), event.event_type);
        }
    }

    fn output_midi(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) {
        // Identify when no controller found yet
        if self.identified_cycles() == 0 {
            self.output().output_message(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        } else if self.identified_cycles() < IDENTIFY_CYCLES {
            self.set_identified_cycles(self.identified_cycles() + 1);
        } else {
            // APC 40 / 20 specific messages
            let mut messages = self.output_messages(cycle, sequencer, surface);

            // Always draw instrument grid
            if surface.instrument_shown() >= Self::INSTRUMENT_OFFSET as usize {
                let instrument = surface.instrument_shown() - Self::INSTRUMENT_OFFSET as usize;
                self.instrument().draw(instrument as u8, 1);
            }
            messages.append(&mut self.instrument().output_messages(0));

            match surface.view {
                View::Instrument => {
                    // Draw zoom grid
                    for index in 0 .. self.zoom_level() { self.solo().draw(index, 1); }
                },
                View::Sequence => {
                
                }
            };

            messages.append(&mut self.solo().output_messages(0));

            // TODO - As all messages are here as one vec, we don't have to use the sorting struct
            // MidiOut anymore
            self.output().output_messages(&mut messages);
        }

        // from this function
        self.output().write_midi(cycle.scope);
    }

    fn output(&mut self) -> &mut MidiOut;
    fn input(&self) -> &jack::Port<jack::MidiIn>;

    fn input_events(&self, scope: &jack::ProcessScope) -> Vec<InputEvent> {
        self.input().iter(scope).map(|message| InputEvent::new(message.time, message.bytes)).collect()
    }

    fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer);
    fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
}

pub struct APC40 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    knob_offset: u8,

    patterns_shown: [u8; 16],
    zoom_level: u8,
    offsets: [u32; 16],
    base_notes: [u8; 16],

    cue_knob: CueKnob,

    grid: Grid,
    side: Side,
    indicator: WideRow,
    instrument: WideRow,
    activator: WideRow,
    solo: WideRow,
    arm: WideRow,
}

impl APC40 {
    fn pattern_shown(&self, index: usize) -> u8 { self.patterns_shown[index] }
}

impl APC for APC40 {
    type Loopable = Pattern;

    const CONTROLLER_ID: u8 = 0;
    const INSTRUMENT_OFFSET: u8 = 8;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 16 / self.zoom_level() as u32 }

    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, level: u8) { self.zoom_level = level }

    fn offset(&self, index: usize) -> u32 { self.offsets[index] }
    fn set_offset(&mut self, index: usize, ticks: u32) { self.offsets[index] = ticks }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let instrument = sequencer.get_instrument(surface.instrument_shown());
        instrument.pattern_mut(self.pattern_shown(surface.instrument_shown()))
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn instrument(&mut self) -> &mut WideRow { &mut self.instrument }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC40 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC40 out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            // Offset knobs by this value to support multiple groups
            knob_offset: 0,

            patterns_shown: [0; 16],
            zoom_level: 4,
            offsets: [0; 16],
            base_notes: [60; 16],

            cue_knob: CueKnob::new(),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            instrument: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            // TODO - Put length indicator here, get length from longest LoopablePatternEvent in phrases?
            arm: WideRow::new(0x30),
        }
    }

    /*
     * Process APC40 specific midi input, shared input is handled by APC trait
     */
    fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        let instrument = sequencer.get_instrument(surface.instrument_shown());
        let pattern = instrument.pattern_mut(self.pattern_shown(surface.instrument_shown()));

        // Only process channel note messages
        match event.event_type {
            InputEventType::FaderMoved { value, fader_type: FaderType::Master } => {
                mixer.master_adjusted(event.time, value);
            },
            InputEventType::KnobTurned { value, knob_type: KnobType::Effect(index) } => {
                // TODO 
                //sequencer.knob_turned(event.time, index + self.knob_offset, value);
            },
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CONTROLLER_ID, button_type);
                let global_modifier = surface.button_memory.global_modifier(button_type);

                match surface.view {
                    View::Instrument => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                // We subtract y from 4 as we want lower notes to be lower on
                                // the grid, the grid counts from the top
                                let offset = self.offset(surface.instrument_shown());
                                // We put base note in center of grid
                                let note = self.base_notes[surface.instrument_shown()] - 2 + (4 - y);

                                if let Some(tick_range) = self.should_add_event(pattern, modifier, x, y, offset, note) {
                                    pattern.try_add_starting_event(LoopableNoteEvent::new(tick_range.start, note, 127));
                                    let mut event = pattern.get_last_event_on_row(note);
                                    event.set_stop(tick_range.stop);
                                    event.stop_velocity = Some(127);

                                    pattern.add_complete_event(event);
                                }
                            },
                            ButtonType::Side(index) => {
                                // TODO - double press logic && recording logic
                                if false {
                                    //instrument.pattern_mut(index).switch_recording_state()
                                } else {
                                    if let Some(ButtonType::Side(modifier_index)) = modifier {
                                        instrument.clone_pattern(modifier_index, index);
                                    } else if let Some(ButtonType::Shift) = global_modifier {
                                        self.set_offset(surface.instrument_shown(), 0);
                                        instrument.pattern_mut(index).clear_events();
                                    } else {
                                        self.patterns_shown[surface.instrument_shown()] = index; 
                                    }
                                }
                            },
                            ButtonType::Activator(index) => {
                                let length = Pattern::minimum_length() * (index as u32 + 1);

                                if pattern.has_explicit_length() && pattern.length() == length {
                                    pattern.unset_length();
                                } else {
                                    pattern.set_length(length);
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
                            ButtonType::Quantization => {
                                // TODO - Move quantizing & quantize_level to "keyboard"
                                //sequencer.switch_quantizing();
                            },
                            _ => (),
                        }
                    },
                    _ => ()
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
                            // TODO timeline
                            //sequencer.queue_sequence(index);
                        } else {
                            surface.toggle_sequence(index);
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage> {
        let mut messages = vec![];

        match surface.view {
            View::Instrument => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Get base note of instrument, as we draw the grid with base note in vertical center
                let base_note = self.base_notes[surface.instrument_shown()];
                let events = loopable.events().iter()
                    .filter(|event| event.note >= base_note - 2 && event.note <= base_note + 2);
                self.draw_events(events, self.offset(surface.instrument_shown()), base_note - 2);

                self.side.draw(4 - self.pattern_shown(surface.instrument_shown()), 1);

                // pattern length selector
                if loopable.has_explicit_length() {
                    for index in 0 .. (loopable.length() / Self::Loopable::minimum_length()) {
                        self.activator.draw(index as u8, 1);
                    }
                }

                // Indicator
                let filters = [InputEvent::is_cue_knob, InputEvent::is_solo_button, InputEvent::is_activator_button];
                messages.append(&mut self.output_indicator(cycle, &filters, surface, loopable.length()));
            },
            View::Sequence => {
                // TODO - Draw sequence stuff
                // TODO - Output sequence indicator
                messages.append(&mut self.indicator.output_messages(0));
            }
        }

        messages.append(&mut self.grid.output_messages(0));
        messages.append(&mut self.side.output_messages(0));
        messages.append(&mut self.activator.output_messages(0));

        messages
    }
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
    indicator: WideRow,
    instrument: WideRow,
    activator: WideRow,
    solo: WideRow,
    arm: WideRow,
}

impl APC20 {
    fn phrase_shown(&self, index: usize) -> u8 { self.phrases_shown[index] }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
}

impl APC for APC20 {
    type Loopable = Phrase;

    const CONTROLLER_ID: u8 = 1;
    const INSTRUMENT_OFFSET: u8 = 0;

    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 4 * 16 / self.zoom_level() as u32 }

    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, level: u8) { self.zoom_level = level }

    fn offset(&self, index: usize) -> u32 { self.offsets[index] }
    fn set_offset(&mut self, index: usize, ticks: u32) { self.offsets[index] = ticks }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let instrument = sequencer.get_instrument(surface.instrument_shown());
        instrument.phrase_mut(self.phrase_shown(surface.instrument_shown()))
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn instrument(&mut self) -> &mut WideRow { &mut self.instrument }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

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
            indicator: WideRow::new(0x34),
            instrument: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            arm: WideRow::new(0x30),
        }
    }

    fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {
        let instrument = sequencer.get_instrument(surface.instrument_shown());
        let phrase = instrument.phrase_mut(self.phrase_shown(surface.instrument_shown()));

        // Only process channel note messages
        match event.event_type {
            // TODO - Use indicator row as fast movement
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CONTROLLER_ID, button_type);

                match surface.view {
                    View::Instrument => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let offset = self.offset(surface.instrument_shown());
                                // We draw grids from bottom to top
                                let pattern = 4 - y;

                                if let Some(tick_range) = self.should_add_event(phrase, modifier, x, y, offset, pattern) {
                                    phrase.try_add_starting_event(LoopablePatternEvent::new(tick_range.start, pattern));
                                    let mut event = phrase.get_last_event_on_row(pattern);
                                    event.set_stop(tick_range.stop);

                                    phrase.add_complete_event(event);
                                }

                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                if let Some(ButtonType::Side(modifier_index)) = modifier {
                                    instrument.clone_phrase(modifier_index, index);
                                } else if let Some(ButtonType::Shift) = global_modifier {
                                    instrument.phrase_mut(index).clear_events();
                                } else {
                                    self.phrases_shown[surface.instrument_shown()] = index;
                                }
                            },
                            ButtonType::Activator(index) => {
                                phrase.set_length(Phrase::default_length() * (index as u32 + 1));
                            },
                            _ => (),
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage> {
        let mut messages = vec![];

        match surface.view {
            View::Instrument => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Draw main grid
                let events = loopable.events().iter();
                self.draw_events(events, self.offset(surface.instrument_shown()), 0);

                // Playable selector
                self.side.draw(4 - self.phrase_shown(surface.instrument_shown()), 1);

                // Length selector
                for index in 0 .. (loopable.length() / Self::Loopable::default_length()) {
                    self.activator.draw(index as u8, 1);
                }

                // Indicator
                let filters = [InputEvent::is_cue_knob, InputEvent::is_solo_button, InputEvent::is_activator_button];
                messages.append(&mut self.output_indicator(cycle, &filters, surface, loopable.length()));
            },
            View::Sequence => {
                // TODO - sequency stuff
                messages.append(&mut self.indicator.output_messages(0));
            }
        }

        messages.append(&mut self.grid.output_messages(0));
        messages.append(&mut self.side.output_messages(0));
        messages.append(&mut self.activator.output_messages(0));
        messages
    }
}
