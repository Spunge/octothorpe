
pub mod input;
mod lights;

use std::ops::Range;
use super::TickRange;
use super::message::{TimedMessage, Message};
use super::cycle::ProcessCycle;
use super::loopable::*;
use super::sequencer::*;
use super::surface::*;
use super::port::MidiOut;
use super::mixer::*;
use super::TimebaseHandler;
use super::events::*;
use input::*;
use lights::*;

const SEQUENCE_COLOR: u8 = 1;
const TIMELINE_COLOR: u8 = 3;
// Wait some cycles for sloooow apc's
const IDENTIFY_CYCLES: u8 = 3;
const LENGTH_INDICATOR_USECS: u64 = 300000;
const PLAYING_LOOPABLE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32;

pub trait APC {
    type Loopable: Loopable;

    const CONTROLLER_ID: u8;
    const TRACK_OFFSET: u8;
    const HEAD_COLOR: u8;
    const TAIL_COLOR: u8;

    fn identified_cycles(&self) -> u8;
    fn set_identified_cycles(&mut self, cycles: u8);
    fn local_id(&self) -> u8;
    fn set_local_id(&mut self, local_id: u8);
    fn device_id(&self) -> u8;
    fn set_device_id(&mut self, device_id: u8);

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
    fn adjusted_offset(&self, track: usize, max_offset: u32, delta_buttons: i8) -> u32 {
        let delta_ticks = delta_buttons as i32 * self.ticks_per_button() as i32;
        let new_offset = self.offset(track) as i32 + delta_ticks;

        if new_offset >= 0 {
            if new_offset < max_offset as i32 { new_offset as u32 } else { max_offset }
        } else { 0 }
    }

    fn shown_loopable_index(&self, surface: &mut Surface) -> u8;
    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable;
    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8>;
    fn loopable_playing_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)>;

    fn cue_knob(&mut self) -> &mut CueKnob;
    fn master(&mut self) -> &mut Single;
    fn grid(&mut self) -> &mut Grid;
    fn side(&mut self) -> &mut Side;
    fn track(&mut self) -> &mut WideRow;
    fn indicator(&mut self) -> &mut WideRow;
    fn activator(&mut self) -> &mut WideRow;
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

    /*
     * Output side indicator, show what patterns/phrases are playing and selected
     */
    fn output_side(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage> {
        // Default to output immediately
        let mut frame = 0;

        if surface.view == View::Track {
            let playing_indexes = self.playing_loopable_indexes(cycle, sequencer, surface);
            let showed_index = self.shown_loopable_index(surface);

            let state = 1 - (cycle.tick_range.start / PLAYING_LOOPABLE_INDICATOR_TICKS) % 2;

            for index in playing_indexes.into_iter() {
                // Playable selector
                self.side().draw(index, state as u8);
            }

            // Playable selector
            self.side().draw(showed_index, 1);

            // Switch on correct frame
            if cycle.tick_range.stop % PLAYING_LOOPABLE_INDICATOR_TICKS < cycle.tick_range.length() {
                frame = (((cycle.tick_range.stop % PLAYING_LOOPABLE_INDICATOR_TICKS) as f64 / cycle.tick_range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
            }
        }

        self.side().output_messages(frame)
    }

    // TODO - only draw length indicator at position 0 only when we are precisely at 0
    fn output_indicator(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage> {
        // Default to output immediately
        let mut frame = 0;
        let loopable_length = self.shown_loopable(sequencer, surface).length();

        if surface.view == View::Track {
            let filters = [InputEvent::is_cue_knob, InputEvent::is_solo_button, InputEvent::is_activator_button];
            let usecs = cycle.time_stop - LENGTH_INDICATOR_USECS;

            let offset_buttons = self.offset(surface.track_shown()) / self.ticks_per_button();

            // TODO - move this timing logic to seperate function when we need it for other things
            // Do we need to draw length indicator, and when?
            if let Some(usecs) = surface.event_memory.last_occurred_event_after(Self::CONTROLLER_ID, &filters, usecs) {
                let usecs_ago = cycle.time_stop - usecs;
                let hide_in_usecs = LENGTH_INDICATOR_USECS - usecs_ago;

                if hide_in_usecs < cycle.usecs() {
                    frame = hide_in_usecs as u32 * cycle.scope.n_frames() / cycle.usecs() as u32;
                } else {
                    let length_buttons = (self.indicator().width() as u32 * self.ticks_in_grid() / loopable_length) as u8;
                    //let length_buttons = self.indicator().width() / (length / self.ticks_in_grid()) as u8;
                    let start_button = offset_buttons as u8 * length_buttons / self.indicator().width();
                    let stop_button = start_button + length_buttons;
                    for index in start_button .. stop_button {
                        self.indicator().draw(index as u8, 1);
                    }
                }
            } else {
                // As we don't have to show any time based indicators, show transport position indicator
                let ranges = self.loopable_playing_ranges(cycle, sequencer, surface);

                for (range, start) in ranges {
                    let ticks_into_playable = (range.stop - start);
                    let button = ticks_into_playable / self.ticks_per_button();

                    //let switch_to_led = (((range.stop - start) as f64 / length as f64) * (length / self.ticks_per_button()) as f64) as u32;
                    if button >= offset_buttons {
                        self.indicator().draw((button - offset_buttons) as u8, 1);
                    }

                    // If transition falls within current cycle, switch on correct frame
                    if range.stop % self.ticks_per_button() < range.length() {
                        frame = (((range.stop % self.ticks_per_button()) as f64 / range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
                    }
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
                    // TODO
                    (event.stop().unwrap() as i32 - offset_x as i32) / button_ticks
                };

                // Flip grid around to show higher notes higher on the grid (for patterns this does not matter)
                let row = event.row(offset_y);

                // Always draw first button head
                self.grid().try_draw(start_button, row, Self::HEAD_COLOR);
                // Draw tail depending on wether this is looping note
                if stop_button >= start_button {
                    self.draw_tail((start_button + 1) .. stop_button, row);
                } else {
                    self.draw_tail((start_button + 1) .. max_button, row);
                    self.draw_tail(0 .. stop_button, row);
                }
            });
    }

    fn draw_timeline(&mut self, playing_sequences: &Vec<PlayingSequence>) {
        
    }

    /*
     * Draw grid that we can use to select what phrases are playing
     */
    fn draw_phrases(&mut self, phrases: &[Option<u8>; 16]) {
        for (index, option) in phrases[Self::TRACK_OFFSET as usize .. (Self::TRACK_OFFSET + 8) as usize].iter().enumerate() {
            if let Some(phrase) = option {
                self.grid().try_draw(index as i32, *phrase, SEQUENCE_COLOR);
            }
        }
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
                    self.set_device_id(device_id);
                    self.set_local_id(local_id);
                    // Make sure we stop inquiring
                    // TODO - Make sure every grid is re-initialized after identifying
                    self.set_identified_cycles(1);
                },
                InputEventType::FaderMoved { value, fader_type: FaderType::Track(index) } => {
                    mixer.fader_adjusted(event.time, index + Self::TRACK_OFFSET, value);
                },
                // TODO - Shift events in loopable to right/left when holding shift
                InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                    let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                    let is_first_turn = surface.event_memory
                        .last_occurred_event_after(Self::CONTROLLER_ID, &[InputEvent::is_cue_knob], usecs)
                        .is_none();

                    let delta_buttons = self.cue_knob().process_turn(value, is_first_turn);
                    let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                    let offset = self.adjusted_offset(surface.track_shown(), max_offset, delta_buttons);
                    self.set_offset(surface.track_shown(), offset);
                },
                InputEventType::ButtonPressed(button_type) => {
                    // Register press in memory to keep track of modifing buttons
                    surface.button_memory.press(Self::CONTROLLER_ID, button_type);
                    let global_modifier = surface.button_memory.global_modifier(button_type);

                    // Do the right thing in the right visualization
                    match surface.view {
                        View::Track => {
                            match button_type {
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    if zoom_level != 7 {
                                        self.set_zoom_level(zoom_level);

                                        // It could happen that we're moved out of range when zooming out
                                        let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                                        if self.offset(surface.track_shown()) > max_offset {
                                            self.set_offset(surface.track_shown(), max_offset);
                                        }
                                    }
                                },
                                _ => (),
                            }
                        },
                        View::Sequence => {
                            let sequence = sequencer.get_sequence(surface.sequence_shown());

                            match button_type {
                                ButtonType::Grid(x, row) => {
                                    let track = (x + Self::TRACK_OFFSET) as usize;
                                    
                                    if let Some(true) = sequence.get_phrase(track).and_then(|phrase| Some(phrase == row)) {
                                        sequence.unset_phrase(track)
                                    } else {
                                        sequence.set_phrase(track, row);
                                    }
                                },
                                ButtonType::Side(index) => {
                                    if let Some(ButtonType::Shift) = global_modifier {
                                        sequence.set_phrases(index);
                                    } else {
                                        surface.show_sequence(index);
                                    }
                                },
                                ButtonType::Activator(track) => {
                                    sequence.toggle_active((track + Self::TRACK_OFFSET) as usize)
                                },
                                _ => (),
                            }
                        },
                        View::Timeline => {
                            match button_type {
                                ButtonType::Side(index) => {
                                    surface.show_sequence(index);
                                    surface.switch_view(View::Sequence);
                                }
                                _ => (),
                            }
                            // TODO - Timeline buttons
                        }
                    }

                    // Independent of current view
                    match button_type {
                        ButtonType::Track(index) => {
                            // Switch to sequence when we click currently shown track button
                            if let (&View::Track, true) = (&surface.view, surface.track_shown() == index as usize) {
                                surface.switch_view(View::Sequence);
                            } else {
                                surface.show_track(index + Self::TRACK_OFFSET);
                                surface.switch_view(View::Track);
                            }
                        },
                        // Switch to timeline, when timeline already shown, switch to track
                        ButtonType::Master => {
                            let view = match surface.view {
                                View::Timeline => View::Track,
                                _ => View::Timeline,
                            };
                            surface.switch_view(view);
                        },
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
        let mut messages = vec![];

        // Identify when no controller found yet
        if self.identified_cycles() == 0 {
            messages.push(TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        } else if self.identified_cycles() < IDENTIFY_CYCLES {
            // Output introduction if APC just responded to inquiry
            if self.identified_cycles() == 1 {
                let message = Message::Introduction([0xF0, 0x47, self.local_id(), self.device_id(), 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                messages.push(TimedMessage::new(0, message));
            }

            self.set_identified_cycles(self.identified_cycles() + 1);
        } else {
            // APC 40 / 20 specific messages
            messages.append(&mut self.output_messages(cycle, sequencer, surface));

            // Always draw track grid
            // This if statement is here to see if we can subtract TRACK_OFFSET
            if surface.track_shown() >= Self::TRACK_OFFSET as usize {
                let track = surface.track_shown() - Self::TRACK_OFFSET as usize;
                self.track().draw(track as u8, 1);
            }
            messages.append(&mut self.track().output_messages(0));

            match surface.view {
                View::Track => {
                    // Draw zoom grid
                    for index in 0 .. self.zoom_level() { self.solo().draw(index, 1); }
                },
                View::Timeline => {
                    self.master().draw(1);
                    self.draw_timeline(&sequencer.timeline.playing_sequences);
                },
                View::Sequence => {
                    let phrases = sequencer.get_sequence(surface.sequence_shown()).phrases();
                    self.draw_phrases(phrases);
                    self.side().draw(surface.sequence_shown() as u8, 1);
                },
            };

            messages.append(&mut self.master().output_messages(0));
            messages.append(&mut self.solo().output_messages(0));
            messages.append(&mut self.grid().output_messages(0));
            messages.append(&mut self.activator().output_messages(0));
            messages.append(&mut self.output_side(cycle, sequencer, surface));
            messages.append(&mut self.output_indicator(cycle, sequencer, surface));
        }

        // from this function
        self.output().write_midi(cycle.scope, &mut messages);
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
    device_id: u8,
    local_id: u8,
    knob_offset: u8,

    patterns_shown: [u8; 16],
    zoom_level: u8,
    offsets: [u32; 16],
    base_notes: [u8; 16],

    cue_knob: CueKnob,
    master: Single,

    grid: Grid,
    side: Side,
    indicator: WideRow,
    track: WideRow,
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
    const TRACK_OFFSET: u8 = 8;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 16 / self.zoom_level() as u32 }

    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, level: u8) { self.zoom_level = level }

    fn offset(&self, index: usize) -> u32 { self.offsets[index] }
    fn set_offset(&mut self, index: usize, ticks: u32) { self.offsets[index] = ticks }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &mut Surface) -> u8 {
        self.pattern_shown(surface.track_shown())
    }

    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let track = sequencer.track_mut(surface.track_shown());
        track.pattern_mut(self.shown_loopable_index(surface))
    }

    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.track_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.track_shown(), phrase_index, sequence_start).into_iter()
                    .map(|(pattern_index, _, _, _, _)| pattern_index)
            })
            .collect()
    }

    fn loopable_playing_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases of currently selected track
        let shown_pattern_index = self.shown_loopable_index(surface);
        let pattern = sequencer.track(surface.track_shown()).pattern(shown_pattern_index);

        sequencer.playing_phrases(surface.track_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.track_shown(), phrase_index, sequence_start).into_iter()
                    .filter(|(pattern_index, _, _, _, _)| *pattern_index == shown_pattern_index)
                    .map(move |(_, absolute_start, relative_range, pattern_event_length, absolute_offset)| {
                        let absolute_range = relative_range.plus(absolute_start);

                        // Make sure indicator loops around when pattern has explicit length
                        let start = if pattern.has_explicit_length() {
                            let length = pattern.length();
                            let iterations = relative_range.start / length;
                            absolute_start + iterations * length
                        } else {
                            absolute_start
                        };

                        (absolute_range, start)
                    })
            })
            .collect()
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn master(&mut self) -> &mut Single { &mut self.master }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn side(&mut self) -> &mut Side { &mut self.side }
    fn track(&mut self) -> &mut WideRow { &mut self.track }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC40 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC40 out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,
            // Offset knobs by this value to support multiple groups
            knob_offset: 0,

            patterns_shown: [0; 16],
            zoom_level: 4,
            offsets: [0; 16],
            base_notes: [60; 16],

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            track: WideRow::new(0x33),
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
        let track = sequencer.track_mut(surface.track_shown());
        let pattern = track.pattern_mut(self.pattern_shown(surface.track_shown()));

        // Only process channel note messages
        match event.event_type {
            InputEventType::FaderMoved { value, fader_type: FaderType::Master } => {
                mixer.master_adjusted(event.time, value);
            },
            InputEventType::KnobTurned { value: _, knob_type: KnobType::Effect(_index) } => {
                // TODO 
                //sequencer.knob_turned(event.time, index + self.knob_offset, value);
            },
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CONTROLLER_ID, button_type);
                let global_modifier = surface.button_memory.global_modifier(button_type);

                match surface.view {
                    View::Track => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                // We subtract y from 4 as we want lower notes to be lower on
                                // the grid, the grid counts from the top
                                let offset = self.offset(surface.track_shown());
                                // We put base note in center of grid
                                let note = self.base_notes[surface.track_shown()] - 2 + y;

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
                                    //track.pattern_mut(index).switch_recording_state()
                                } else {
                                    if let Some(ButtonType::Side(modifier_index)) = modifier {
                                        track.clone_pattern(modifier_index, index);
                                    } else if let Some(ButtonType::Shift) = global_modifier {
                                        self.set_offset(surface.track_shown(), 0);
                                        track.pattern_mut(index).clear_events();
                                    } else {
                                        self.patterns_shown[surface.track_shown()] = index; 
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
                                let base_note = &mut self.base_notes[surface.track_shown()];
                                let new_base_note = *base_note + 4;

                                if new_base_note <= 118 { *base_note = new_base_note }
                            },
                            ButtonType::Down => {
                                let base_note = &mut self.base_notes[surface.track_shown()];
                                let new_base_note = *base_note - 4;

                                if new_base_note >= 22 { *base_note = new_base_note }
                            },
                            ButtonType::Right => {
                                let ticks_per_button = self.ticks_per_button();
                                let offset = &mut self.offsets[surface.track_shown()];
                                // There's 8 buttons, shift view one gridwidth to the right
                                *offset = *offset + ticks_per_button * 8;
                            },
                            ButtonType::Left => {
                                let ticks_per_button = self.ticks_per_button();
                                let offset = &mut self.offsets[surface.track_shown()];
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
                    ButtonType::Play => sequencer.start(cycle),
                    ButtonType::Stop => {
                        // Reset to 0 when we press stop button but we're already stopped
                        let (state, _) = cycle.client.transport_query();
                        match state {
                            1 => sequencer.stop(cycle),
                            _ => sequencer.reset(cycle),
                        };
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
            View::Track => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Get base note of track, as we draw the grid with base note in vertical center
                let base_note = self.base_notes[surface.track_shown()];
                let events = loopable.events().iter()
                    .filter(|event| event.note >= base_note - 2 && event.note <= base_note + 2);
                self.draw_events(events, self.offset(surface.track_shown()), base_note - 2);

                // pattern length selector
                if loopable.has_explicit_length() {
                    for index in 0 .. (loopable.length() / Self::Loopable::minimum_length()) {
                        self.activator.draw(index as u8, 1);
                    }
                }
            },
            View::Sequence => {
                // TODO - Draw sequence stuff
                // TODO - Output sequence indicator
            },
            View::Timeline => {
            
            }
        }

        messages
    }
}


pub struct APC20 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    device_id: u8,
    local_id: u8,

    phrases_shown: [u8; 16],
    zoom_level: u8,
    offsets: [u32; 16],

    cue_knob: CueKnob,
    master: Single,

    // Lights
    grid: Grid,
    side: Side,
    indicator: WideRow,
    track: WideRow,
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
    const TRACK_OFFSET: u8 = 0;

    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn ticks_in_grid(&self) -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 4 * 16 / self.zoom_level() as u32 }

    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, level: u8) { self.zoom_level = level }

    fn offset(&self, index: usize) -> u32 { self.offsets[index] }
    fn set_offset(&mut self, index: usize, ticks: u32) { self.offsets[index] = ticks }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &mut Surface) -> u8 {
        self.phrase_shown(surface.track_shown())
    }

    fn shown_loopable<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let track = sequencer.track_mut(surface.track_shown());
        track.phrase_mut(self.shown_loopable_index(surface))
    }

    // Get indexes of currently playing phrases in showed track
    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.track_shown(), &cycle.tick_range).into_iter()
            .map(|(_, _, phrase_index)| phrase_index)
            .collect()
    }

    fn loopable_playing_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases for currently selected track
        let shown_phrase_index = self.shown_loopable_index(surface);
        let length = sequencer.track(surface.track_shown()).phrase(shown_phrase_index).length();

        sequencer.playing_phrases(surface.track_shown(), &cycle.tick_range).into_iter()
            .filter(|(_, _, index)| *index == shown_phrase_index)
            .map(|(range, sequence_start, _)| {
                let iterations = (range.start - sequence_start) / length;

                (range, sequence_start + iterations * length)
            })
            .collect()
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn master(&mut self) -> &mut Single { &mut self.master }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn side(&mut self) -> &mut Side { &mut self.side }
    fn track(&mut self) -> &mut WideRow { &mut self.track }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("APC20 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("APC20 out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,

            phrases_shown: [0; 16],
            zoom_level: 4,
            offsets: [0; 16],

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            track: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            arm: WideRow::new(0x30),
        }
    }

    fn process_inputevent(&mut self, event: &InputEvent, _cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, _mixer: &mut Mixer) {
        let track = sequencer.track_mut(surface.track_shown());
        let phrase = track.phrase_mut(self.phrase_shown(surface.track_shown()));

        // Only process channel note messages
        match event.event_type {
            // TODO - Use indicator row as fast movement
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CONTROLLER_ID, button_type);

                match surface.view {
                    View::Track => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let offset = self.offset(surface.track_shown());
                                // We draw grids from bottom to top

                                if let Some(tick_range) = self.should_add_event(phrase, modifier, x, y, offset, y) {
                                    phrase.try_add_starting_event(LoopablePatternEvent::new(tick_range.start, y));
                                    let mut event = phrase.get_last_event_on_row(y);
                                    event.set_stop(tick_range.stop);

                                    phrase.add_complete_event(event);
                                }

                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                if let Some(ButtonType::Side(modifier_index)) = modifier {
                                    track.clone_phrase(modifier_index, index);
                                } else if let Some(ButtonType::Shift) = global_modifier {
                                    track.phrase_mut(index).clear_events();
                                } else {
                                    self.phrases_shown[surface.track_shown()] = index;
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
            View::Track => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Draw main grid
                let events = loopable.events().iter();
                self.draw_events(events, self.offset(surface.track_shown()), 0);

                // Length selector
                for index in 0 .. (loopable.length() / Self::Loopable::default_length()) {
                    self.activator.draw(index as u8, 1);
                }
            },
            View::Sequence => {
            },
            View::Timeline => {
            }
        }

        messages
    }
}
