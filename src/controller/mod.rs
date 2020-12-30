
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
const TIMELINE_HEAD_COLOR: u8 = 1;
const TIMELINE_TAIL_COLOR: u8 = 3;
// Same as default phrase length atm
// Wait some cycles for sloooow apc's
const IDENTIFY_CYCLES: u8 = 3;
const LENGTH_INDICATOR_USECS: u64 = 200000;
const DOUBLE_CLICK_USECS: u64 = 300000;
const PLAYING_LOOPABLE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32;
const PLAYING_SEQUENCE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32;
const QUEUED_SEQUENCE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32 / 2;

pub trait APC {
    type Loopable: Loopable;

    const TRACK_OFFSET: u8;
    const HEAD_COLOR: u8;
    const TAIL_COLOR: u8;

    fn identified_cycles(&self) -> u8;
    fn set_identified_cycles(&mut self, cycles: u8);
    fn local_id(&self) -> u8;
    fn set_local_id(&mut self, local_id: u8);
    fn device_id(&self) -> u8;
    fn set_device_id(&mut self, device_id: u8);

    fn loopable_ticks_per_button(&self, surface: &Surface) -> u32;
    fn loopable_ticks_in_grid(&self, surface: &Surface) -> u32;
    fn loopable_zoom_level(&self, surface: &Surface) -> u8;
    fn set_loopable_zoom_level(&self, sequencer: &Sequencer, surface: &mut Surface, zoom_level: u8);
    fn shown_loopable_offset(&self, surface: &Surface) -> u32;
    fn set_shown_loopable_offset(&self, sequencer: &Sequencer, surface: &mut Surface, offset: u32);

    fn shown_loopable_index(&self, surface: &Surface) -> u8;
    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, surface: &Surface) -> &'a Self::Loopable;
    fn shown_loopable_mut<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable;
    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8>;
    fn playing_loopable_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)>;

    fn cue_knob(&mut self) -> &mut CueKnob;
    fn master(&mut self) -> &mut Single;
    fn grid(&mut self) -> &mut Grid;
    fn side(&mut self) -> &mut Side;
    fn channel(&mut self) -> &mut WideRow;
    fn indicator(&mut self) -> &mut WideRow;
    fn activator(&mut self) -> &mut WideRow;
    fn solo(&mut self) -> &mut WideRow;

    /*
     * Remove existing events when there's starting events in tick range, otherwise, remove tick
     * range so we can add new event
     */
    fn should_add_event(&self, loopable: &mut impl Loopable, modifier: Option<ButtonType>, ticks_per_button: u32, x: u8, y: u8, offset: u32, row: u8) -> Option<TickRange> {
        let start = x as u32 * ticks_per_button + offset;
        let mut tick_range = TickRange::new(start, start + ticks_per_button);

        // Should we delete the event we're clicking?
        if let (None, true) = (modifier, loopable.contains_events_starting_in(tick_range, row)) {
            loopable.remove_events_starting_in(tick_range, row);
            None
        } else {
            // Add event get x from modifier when its a grid button in the same row
            if let Some(ButtonType::Grid(mod_x, mod_y)) = modifier {
                if mod_y == y { 
                    tick_range.start = mod_x as u32 * ticks_per_button + offset;
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

        match surface.view {
            View::Channel => {
                let playing_indexes = self.playing_loopable_indexes(cycle, sequencer, surface);
                let showed_index = self.shown_loopable_index(surface);

                let state = 1 - (cycle.tick_range.start / PLAYING_LOOPABLE_INDICATOR_TICKS) % 2;

                // Draw blinking playing loopables
                for index in playing_indexes.into_iter() {
                    self.side().draw(index, state as u8);
                }

                // Always show selected loopable
                self.side().draw(showed_index, 1);

                // Switch on correct frame
                if cycle.tick_range.stop % PLAYING_LOOPABLE_INDICATOR_TICKS < cycle.tick_range.length() {
                    frame = (((cycle.tick_range.stop % PLAYING_LOOPABLE_INDICATOR_TICKS) as f64 / cycle.tick_range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
                }
            },
            View::Sequence => {
                // Draw blinking playing sequences
                let playing_state = 1 - (cycle.tick_range.start / PLAYING_SEQUENCE_INDICATOR_TICKS) % 2;
                self.side().draw(sequencer.sequence_playing as u8, playing_state as u8);

                // Playable selector
                self.side().draw(surface.sequence_shown() as u8, 1);

                // If theres something queued, make sure that blinks like crazy
                if let Some(index) = sequencer.sequence_queued {
                    let queued_state = 1 - (cycle.tick_range.start / QUEUED_SEQUENCE_INDICATOR_TICKS) % 2;
                    self.side().draw(index as u8, queued_state as u8);
                }

                // Switch on correct frame
                if cycle.tick_range.stop % PLAYING_SEQUENCE_INDICATOR_TICKS < cycle.tick_range.length() {
                    frame = (((cycle.tick_range.stop % PLAYING_SEQUENCE_INDICATOR_TICKS) as f64 / cycle.tick_range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
                }
            },
            _ => (),
        }

        self.side().output_messages(frame)
    }

    // TODO - only draw length indicator at position 0 only when we are precisely at 0
    fn output_indicator(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage> {
        // Default to output immediately
        let mut frame = 0;
        let loopable_length = self.shown_loopable(sequencer, surface).length();

        match surface.view {
            View::Channel => {
                let usecs = cycle.time_stop - LENGTH_INDICATOR_USECS;
                let ticks_per_button = self.loopable_ticks_per_button(surface);
                let offset_buttons = self.shown_loopable_offset(surface) / ticks_per_button;
                let controller_filters = [
                    InputEvent::is_cue_knob,
                    InputEvent::is_solo_button,
                    InputEvent::is_activator_button,
                    InputEvent::is_right_button,
                    InputEvent::is_left_button
                ];
                let global_filters = [InputEvent::is_crossfader];
                // Show length/offset indicator when events occurred that changed length/offset
                let last_occurred_controller_event = surface.event_memory
                    .last_occurred_controller_event_after(Self::TRACK_OFFSET, &controller_filters, usecs)
                    .or_else(|| surface.event_memory.last_occurred_global_event_after(&global_filters, usecs));

                // TODO - move this timing logic to seperate function when we need it for other things
                // Do we need to draw length indicator, and when?
                if let Some(usecs) = last_occurred_controller_event {
                    let usecs_ago = cycle.time_stop - usecs;
                    let hide_in_usecs = LENGTH_INDICATOR_USECS - usecs_ago;

                    if hide_in_usecs < cycle.usecs() {
                        frame = hide_in_usecs as u32 * cycle.scope.n_frames() / cycle.usecs() as u32;
                    } else {
                        let length_buttons = (self.indicator().width() as u32 * self.loopable_ticks_in_grid(surface) / loopable_length) as u8;
                        let start_button = offset_buttons as u8 * length_buttons / self.indicator().width();
                        let stop_button = start_button + length_buttons;
                        for index in start_button .. stop_button {
                            self.indicator().draw(index as u8, 1);
                        }
                    }
                } else {
                    // As we don't have to show any time based indicators, show transport position indicator
                    let ranges = self.playing_loopable_ranges(cycle, sequencer, surface);

                    for (range, start) in ranges {
                        let ticks_into_playable = range.stop - start;
                        let button = ticks_into_playable / ticks_per_button;

                        if button >= offset_buttons {
                            self.indicator().draw((button - offset_buttons) as u8, 1);
                        }

                        // If transition falls within current cycle, switch on correct frame
                        if range.stop % ticks_per_button < range.length() {
                            frame = (((range.stop % ticks_per_button) as f64 / range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
                        }
                    }
                }
            },
            View::Timeline => {
                let button = cycle.tick_range.start / Surface::TIMELINE_TICKS_PER_BUTTON;
                let offset_buttons = surface.timeline_offset() / Surface::TIMELINE_TICKS_PER_BUTTON + Self::TRACK_OFFSET as u32;

                if button >= offset_buttons {
                    self.indicator().draw((button - offset_buttons) as u8, 1);
                }

                // If transition falls within current cycle, switch on correct frame
                if cycle.tick_range.stop % Surface::TIMELINE_TICKS_PER_BUTTON < cycle.tick_range.length() {
                    frame = (((cycle.tick_range.stop % Surface::TIMELINE_TICKS_PER_BUTTON) as f64 / cycle.tick_range.length() as f64) * cycle.scope.n_frames() as f64) as u32;
                }
            },
            _ => (),
        }

        self.indicator().output_messages(frame)
    }

    /*
     * Draw note or pattern events into main grid of controller
     */
    fn draw_loopable_events<'a>(&mut self, events: impl Iterator<Item = &'a (impl LoopableEvent + 'a)>, 
        offset_x: u32, offset_y: u8, ticks_in_grid: u32, head_color: u8, tail_color: u8) 
    {
        let grid_stop = offset_x + ticks_in_grid;
        let ticks_per_button = (ticks_in_grid / 8) as i32;

        // Draw main grid
        events
            .filter(|event| { 
                let grid_contains_event = event.start() < grid_stop 
                    && (event.stop().is_none() || event.stop().unwrap() > offset_x);

                grid_contains_event || event.is_looping()
            })
            .for_each(|event| {
                // Get buttons from event ticks
                let max_button = self.grid().width() as i32;
                let start_button = (event.start() as i32 - offset_x as i32) / ticks_per_button;
                let stop_button = if event.stop().is_none() { 
                    start_button + 1
                } else { 
                    // Could be event is to short for 1 button, in that case, draw 1 button
                    // TODO
                    (event.stop().unwrap() as i32 - offset_x as i32) / ticks_per_button
                };

                // Flip grid around to show higher notes higher on the grid (for patterns this does not matter)
                let row = event.row(offset_y);

                // Always draw first button head
                self.grid().try_draw(start_button, row, head_color);
                // Draw tail depending on wether this is looping note
                if stop_button >= start_button {
                    self.draw_tail((start_button + 1) .. stop_button, row, tail_color);
                } else {
                    self.draw_tail((start_button + 1) .. max_button, row, tail_color);
                    self.draw_tail(0 .. stop_button, row, tail_color);
                }
            });
    }

    fn draw_timeline(&mut self, sequencer: &Sequencer, surface: &Surface) {
        let channel = sequencer.channel(surface.channel_shown());

        // Draw main grid
        let events = channel.timeline.events().iter();
        let offset = Surface::TIMELINE_TICKS_PER_BUTTON * Self::TRACK_OFFSET as u32 + surface.timeline_offset();
        self.draw_loopable_events(events, offset, 0, Surface::TIMELINE_TICKS_PER_BUTTON * 8, TIMELINE_HEAD_COLOR, TIMELINE_TAIL_COLOR);
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

    fn draw_tail(&mut self, mut x_range: Range<i32>, y: u8, color: u8) {
        while let Some(x) = x_range.next() { self.grid().try_draw(x, y, color) }
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
                InputEventType::FaderMoved { value, fader_type: FaderType::Channel(index) } => {
                    mixer.fader_adjusted(event.time, index + Self::TRACK_OFFSET, value);
                },
                // TODO - Shift events in loopable to right/left when holding shift
                InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                    // Check if cueknob should respond immediately
                    let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                    let is_first_turn = surface.event_memory
                        .last_occurred_controller_event_after(Self::TRACK_OFFSET, &[InputEvent::is_cue_knob], usecs)
                        .is_none();

                    let delta_buttons = self.cue_knob().process_turn(value, is_first_turn);

                    match surface.view {
                        View::Channel => {
                            let delta_ticks = delta_buttons as i32 * self.loopable_ticks_per_button(surface) as i32;
                            let new_offset = self.shown_loopable_offset(surface) as i32 + delta_ticks;
                            let offset = if new_offset < 0 { 0 } else { new_offset as u32 };

                            self.set_shown_loopable_offset(sequencer, surface, offset);
                        },
                        View::Timeline => {
                            let new_offset = surface.timeline_offset() as i32 + (delta_buttons as i32 * Surface::TIMELINE_TICKS_PER_BUTTON as i32);

                            if new_offset >= 0 {
                                surface.set_timeline_offset(sequencer, new_offset as u32);
                            }
                        },
                        _ => (),
                    }
                },
                InputEventType::ButtonPressed(button_type) => {
                    // Register press in memory to keep channel of modifing buttons
                    surface.button_memory.press(Self::TRACK_OFFSET, button_type);
                    let global_modifier = surface.button_memory.global_modifier(button_type);

                    // Do the right thing in the right visualization
                    match surface.view {
                        View::Channel => {
                            match button_type {
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    if zoom_level != 7 {
                                        self.set_loopable_zoom_level(sequencer, surface, zoom_level);
                                    }
                                },
                                _ => (),
                            }
                        },
                        View::Sequence => {
                            let sequence = sequencer.get_sequence(surface.sequence_shown());

                            match button_type {
                                ButtonType::Grid(x, row) => {
                                    let channel = (x + Self::TRACK_OFFSET) as usize;
                                    
                                    if let Some(true) = sequence.get_phrase(channel).and_then(|phrase| Some(phrase == row)) {
                                        sequence.unset_phrase(channel)
                                    } else {
                                        sequence.set_phrase(channel, row);
                                    }
                                },
                                ButtonType::Side(index) => {
                                    // TODO - Move double click logic to surface
                                    let filters = vec![|event_type: &InputEventType| -> bool {
                                        *event_type == event.event_type
                                    }];
                                    let usecs = cycle.time_stop - DOUBLE_CLICK_USECS;
                                    let last_occurred_event = surface.event_memory.last_occurred_controller_event_after(Self::TRACK_OFFSET, &filters, usecs);

                                    if let Some(ButtonPress { button_type: ButtonType::Shift, .. }) = global_modifier {
                                        sequence.set_phrases(index);
                                    } else if let Some(_) = last_occurred_event {
                                        // If we double clicked sequence button, queue it
                                        sequencer.sequence_queued = Some(index as usize);
                                    } else {
                                        surface.show_sequence(index);
                                    }
                                },
                                ButtonType::Activator(channel) => {
                                    sequence.toggle_active((channel + Self::TRACK_OFFSET) as usize)
                                },
                                _ => (),
                            }
                        },
                        View::Timeline => {
                            match button_type {
                                ButtonType::Grid(x, y) => {
                                    let channel = sequencer.channel_mut(surface.channel_shown());

                                    // Add channel offset to make it possible to draw across multiple controllers
                                    let start = (Self::TRACK_OFFSET + x) as u32 * Surface::TIMELINE_TICKS_PER_BUTTON + surface.timeline_offset();
                                    let mut tick_range = TickRange::new(start, start + Surface::TIMELINE_TICKS_PER_BUTTON);

                                    // Should we delete the event we're clicking?
                                    if let (None, true) = (global_modifier, channel.timeline.contains_events_starting_in(tick_range, y)) {
                                        channel.timeline.remove_events_starting_in(tick_range, y);
                                    } else {
                                        // Add event get x from modifier when its a grid button in the same row
                                        if let Some(ButtonPress { button_type: ButtonType::Grid(mod_x, mod_y), controller_channel_offset }) = global_modifier {
                                            if *mod_y == y { 
                                                // Add channel offset off modifier to make it possible to draw across controllers
                                                tick_range.start = (mod_x + controller_channel_offset) as u32 * Surface::TIMELINE_TICKS_PER_BUTTON + surface.timeline_offset();
                                            }
                                        }

                                        // Switch start & stop if stop button was pressed before start button
                                        if tick_range.start > tick_range.stop {
                                            let start = tick_range.start;
                                            // As stop is @ end of button & start is @ start of
                                            // button, offset start & stop by a button
                                            tick_range.start = tick_range.stop - Surface::TIMELINE_TICKS_PER_BUTTON;
                                            tick_range.stop = start + Surface::TIMELINE_TICKS_PER_BUTTON;
                                        }

                                        channel.timeline.add_complete_event(LoopablePhraseEvent::new(tick_range.start, tick_range.stop, y));
                                    }
                                },
                                _ => (),
                            }
                        }
                    }

                    // Independent of current view
                    match button_type {
                        ButtonType::Channel(index) => {
                            match surface.view {
                                View::Channel | View::Timeline => {
                                    if surface.channel_shown() == index as usize {
                                        let view = if matches!(surface.view, View::Timeline) { View::Channel } else { View::Timeline };
                                        surface.switch_view(view);
                                    } else {
                                        surface.show_channel(index + Self::TRACK_OFFSET);
                                    }
                                },
                                _ => {
                                    surface.switch_view(View::Timeline);
                                    surface.show_channel(index + Self::TRACK_OFFSET);
                                },
                            }
                        },
                        // Switch to timeline, when timeline already shown, switch to channel
                        ButtonType::Master => {
                            let view = match surface.view {
                                View::Sequence => View::Channel,
                                _ => View::Sequence,
                            };
                            surface.switch_view(view);
                        },
                        _ => self.process_inputevent(&event, cycle, sequencer, surface, mixer),
                    }
                },
                InputEventType::ButtonReleased(button_type) => {
                    surface.button_memory.release(Self::TRACK_OFFSET, cycle.time_at_frame(event.time), button_type);
                },
                // This message is controller specific, handle it accordingly
                _ => self.process_inputevent(&event, cycle, sequencer, surface, mixer),
            }

            // Keep channel of event so we can use it to calculate double presses etc.
            surface.event_memory.register_event(Self::TRACK_OFFSET, cycle.time_at_frame(event.time), event.event_type);
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
            self.draw(sequencer, surface);

            // Always draw channel grid
            // This if statement is here to see if we can subtract TRACK_OFFSET
            if surface.channel_shown() >= Self::TRACK_OFFSET as usize && ! matches!(surface.view, View::Sequence) {
                let channel = surface.channel_shown() - Self::TRACK_OFFSET as usize;
                self.channel().draw(channel as u8, 1);
            }
            messages.append(&mut self.channel().output_messages(0));

            match surface.view {
                View::Channel => {
                    // Draw zoom grid
                    for index in 0 .. self.loopable_zoom_level(surface) { self.solo().draw(index, 1); }
                },
                View::Timeline => {
                    self.draw_timeline(sequencer, surface);
                },
                View::Sequence => {
                    self.master().draw(1);
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
    fn draw(&mut self, sequencer: &mut Sequencer, surface: &mut Surface);
}

pub struct APC40 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    device_id: u8,
    local_id: u8,
    //knob_offset: u8,

    cue_knob: CueKnob,
    master: Single,

    grid: Grid,
    side: Side,
    indicator: WideRow,
    channel: WideRow,
    activator: WideRow,
    solo: WideRow,
    //arm: WideRow,
}

impl APC for APC40 {
    type Loopable = Pattern;

    const TRACK_OFFSET: u8 = 8;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn loopable_ticks_per_button(&self, surface: &Surface) -> u32 { surface.pattern_ticks_per_button() }
    fn loopable_ticks_in_grid(&self, surface: &Surface) -> u32 { surface.pattern_ticks_in_grid() }
    fn loopable_zoom_level(&self, surface: &Surface) -> u8 { surface.pattern_zoom_level() }
    fn set_loopable_zoom_level(&self, sequencer: &Sequencer, surface: &mut Surface, zoom_level: u8) { 
        surface.set_pattern_zoom_level(sequencer, zoom_level);
    }
    fn shown_loopable_offset(&self, surface: &Surface) -> u32 { surface.pattern_offset(surface.channel_shown()) }
    fn set_shown_loopable_offset(&self, sequencer: &Sequencer, surface: &mut Surface, offset: u32) { 
        surface.set_pattern_offset(sequencer, surface.channel_shown(), offset) 
    }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &Surface) -> u8 { surface.pattern_shown(surface.channel_shown()) }

    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, surface: &Surface) -> &'a Self::Loopable { 
        let channel = sequencer.channel(surface.channel_shown());
        channel.pattern(self.shown_loopable_index(surface))
    }
    fn shown_loopable_mut<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let channel = sequencer.channel_mut(surface.channel_shown());
        channel.pattern_mut(self.shown_loopable_index(surface))
    }

    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.channel_shown(), phrase_index, sequence_start).into_iter()
                    .map(|(pattern_index, _, _, _, _)| pattern_index)
            })
            .collect()
    }

    fn playing_loopable_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases of currently selected channel
        let shown_pattern_index = self.shown_loopable_index(surface);
        let pattern = sequencer.channel(surface.channel_shown()).pattern(shown_pattern_index);

        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.channel_shown(), phrase_index, sequence_start).into_iter()
                    .filter(|(pattern_index, _, _, _, _)| *pattern_index == shown_pattern_index)
                    .map(move |(_, absolute_start, relative_range, _, _)| {
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
    fn channel(&mut self) -> &mut WideRow { &mut self.channel }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("apc40_in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("apc40_out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,
            // Offset knobs by this value to support multiple groups
            //knob_offset: 0,

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            channel: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            // TODO - Put length indicator here, get length from longest LoopablePatternEvent in phrases?
            //arm: WideRow::new(0x30),
        }
    }

    /*
     * Process APC40 specific midi input, shared input is handled by APC trait
     */
    fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, mixer: &mut Mixer) {

        // Only process channel note messages
        match event.event_type {
            InputEventType::FaderMoved { value, fader_type: FaderType::Master } => {
                mixer.master_adjusted(event.time, value);
            },
            InputEventType::FaderMoved { value, fader_type: FaderType::CrossFade } => {
                let factor = value as f64 / 127.0;
                //let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                
                //let offset = (max_offset as f64 * factor) as u32;
                surface.set_offsets_by_factor(sequencer, surface.channel_shown(), factor);
                //println!("{:?} {:?}", offset, max_offset);
                
                //self.set_offset(surface.channel_shown(), offset);
                //mixer.master_adjusted(event.time, value);
            },
            InputEventType::KnobTurned { value: _, knob_type: KnobType::Effect(_index) } => {
                // TODO 
                //sequencer.knob_turned(event.time, index + self.knob_offset, value);
            },
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::TRACK_OFFSET, button_type);

                match surface.view {
                    View::Channel => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let channel = sequencer.channel_mut(surface.channel_shown());
                                let pattern = channel.pattern_mut(surface.pattern_shown(surface.channel_shown()));

                                // We subtract y from 4 as we want lower notes to be lower on
                                // the grid, the grid counts from the top
                                let offset = surface.pattern_offset(surface.channel_shown());
                                // We put base note in center of grid
                                let note = surface.pattern_base_note(surface.channel_shown()) - 2 + y;
                                let ticks_per_button = self.loopable_ticks_per_button(surface);

                                if let Some(tick_range) = self.should_add_event(pattern, modifier, ticks_per_button, x, y, offset, note) {
                                    pattern.try_add_starting_event(LoopableNoteEvent::new(tick_range.start, note, 127));
                                    let mut event = pattern.get_last_event_on_row(note);
                                    event.set_stop(tick_range.stop);
                                    event.stop_velocity = Some(127);

                                    pattern.add_complete_event(event);
                                }
                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                // TODO - double press logic && recording logic
                                if false {
                                    //channel.pattern_mut(index).switch_recording_state()
                                } else {
                                    if let Some(ButtonType::Side(modifier_index)) = modifier {
                                        let channel = sequencer.channel_mut(surface.channel_shown());
                                        channel.clone_pattern(modifier_index, index);
                                    } else if let Some(ButtonPress { button_type: ButtonType::Shift, .. }) = global_modifier {
                                        surface.set_pattern_offset(sequencer, surface.channel_shown(), 0);

                                        let channel = sequencer.channel_mut(surface.channel_shown());
                                        channel.pattern_mut(index).clear_events();
                                    } else {
                                        surface.show_pattern(surface.channel_shown(), index);
                                    }
                                }
                            },
                            ButtonType::Activator(index) => {
                                let channel = sequencer.channel_mut(surface.channel_shown());
                                let pattern = channel.pattern_mut(surface.pattern_shown(surface.channel_shown()));
                                let length = Pattern::minimum_length() * (index as u32 + 1);

                                if pattern.has_explicit_length() && pattern.length() == length {
                                    pattern.unset_length();
                                } else {
                                    pattern.set_length(length);
                                }
                            },
                            ButtonType::Up => {
                                let base_note = surface.pattern_base_note(surface.channel_shown());
                                surface.set_pattern_base_note(surface.channel_shown(), base_note + 4);
                            },
                            ButtonType::Down => {
                                let base_note = surface.pattern_base_note(surface.channel_shown());
                                surface.set_pattern_base_note(surface.channel_shown(), base_note - 4) 
                            },
                            ButtonType::Right => {
                                let ticks_per_button = self.loopable_ticks_per_button(surface);
                                let offset = surface.pattern_offset(surface.channel_shown());
                                // There's 8 buttons, shift view one gridwidth to the right
                                surface.set_pattern_offset(sequencer, surface.channel_shown(), offset + ticks_per_button * 8);
                            },
                            ButtonType::Left => {
                                let ticks_per_button = self.loopable_ticks_per_button(surface);
                                let offset = surface.pattern_offset(surface.channel_shown());
                                let new_offset = offset as i32 - (ticks_per_button * 8) as i32;
                                let offset = if new_offset >= 0 { new_offset as u32 } else { 0 };

                                surface.set_pattern_offset(sequencer, surface.channel_shown(), offset);
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
                        let (state, pos) = cycle.client.transport_query();
                        let is_transport_at_start = pos.bar == 1 && pos.beat == 1 && pos.tick == 0;
                        let global_modifier = surface.button_memory.global_modifier(button_type);

                        // Reset timeline when we shift press stop @ 0:0:0
                        if let (Some(ButtonPress { button_type: ButtonType::Shift, .. }), true) = (global_modifier, is_transport_at_start) {
                            sequencer.reset_timeline();
                        } else {
                            match state {
                                1 => sequencer.stop(cycle),
                                _ => {
                                    sequencer.reset(cycle);
                                    surface.set_timeline_offset(sequencer, 0);
                                },
                            };
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    fn draw(&mut self, sequencer: &mut Sequencer, surface: &mut Surface) {
        match surface.view {
            View::Channel => {
                let loopable = self.shown_loopable_mut(sequencer, surface);

                // Get base note of channel, as we draw the grid with base note in vertical center
                let base_note = surface.pattern_base_note(surface.channel_shown());
                let events = loopable.events().iter()
                    .filter(|event| event.note >= base_note - 2 && event.note <= base_note + 2);

                self.draw_loopable_events(events, surface.pattern_offset(surface.channel_shown()), base_note - 2, self.loopable_ticks_in_grid(surface), Self::HEAD_COLOR, Self::TAIL_COLOR);

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
    }
}


pub struct APC20 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    device_id: u8,
    local_id: u8,

    cue_knob: CueKnob,
    master: Single,

    // Lights
    grid: Grid,
    side: Side,
    indicator: WideRow,
    channel: WideRow,
    activator: WideRow,
    solo: WideRow,
    //arm: WideRow,
}

impl APC for APC20 {
    type Loopable = Phrase;

    const TRACK_OFFSET: u8 = 0;
    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn loopable_ticks_per_button(&self, surface: &Surface) -> u32 { surface.phrase_ticks_per_button() }
    fn loopable_ticks_in_grid(&self, surface: &Surface) -> u32 { surface.phrase_ticks_in_grid() }
    fn loopable_zoom_level(&self, surface: &Surface) -> u8 { surface.phrase_zoom_level() }
    fn set_loopable_zoom_level(&self, sequencer: &Sequencer, surface: &mut Surface, zoom_level: u8) { 
        surface.set_phrase_zoom_level(sequencer, zoom_level);
    }
    fn shown_loopable_offset(&self, surface: &Surface) -> u32 { surface.phrase_offset(surface.channel_shown()) }
    fn set_shown_loopable_offset(&self, sequencer: &Sequencer, surface: &mut Surface, offset: u32) {
        surface.set_phrase_offset(sequencer, surface.channel_shown(), offset);
    }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &Surface) -> u8 { surface.phrase_shown(surface.channel_shown()) }

    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, surface: &Surface) -> &'a Self::Loopable { 
        let channel = sequencer.channel(surface.channel_shown());
        channel.phrase(self.shown_loopable_index(surface))
    }
    fn shown_loopable_mut<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let channel = sequencer.channel_mut(surface.channel_shown());
        channel.phrase_mut(self.shown_loopable_index(surface))
    }

    // Get indexes of currently playing phrases in showed channel
    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .map(|(_, _, phrase_index)| phrase_index)
            .collect()
    }

    fn playing_loopable_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases for currently selected channel
        let shown_phrase_index = self.shown_loopable_index(surface);
        let length = sequencer.channel(surface.channel_shown()).phrase(shown_phrase_index).length();

        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
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
    fn channel(&mut self) -> &mut WideRow { &mut self.channel }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("apc20_in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("apc20_out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            channel: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            //arm: WideRow::new(0x30),
        }
    }

    fn process_inputevent(&mut self, event: &InputEvent, _cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface, _mixer: &mut Mixer) {
        let channel = sequencer.channel_mut(surface.channel_shown());
        let phrase = channel.phrase_mut(surface.phrase_shown(surface.channel_shown()));

        // Only process channel note messages
        match event.event_type {
            // TODO - Use indicator row as fast movement
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::TRACK_OFFSET, button_type);

                match surface.view {
                    View::Channel => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let offset = surface.phrase_offset(surface.channel_shown());
                                // We draw grids from bottom to top
                                let ticks_per_button = self.loopable_ticks_per_button(surface);

                                if let Some(tick_range) = self.should_add_event(phrase, modifier, ticks_per_button, x, y, offset, y) {
                                    phrase.try_add_starting_event(LoopablePatternEvent::new(tick_range.start, y));
                                    let mut event = phrase.get_last_event_on_row(y);
                                    event.set_stop(tick_range.stop);

                                    phrase.add_complete_event(event);
                                }
                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                if let Some(ButtonType::Side(modifier_index)) = modifier {
                                    channel.clone_phrase(modifier_index, index);
                                } else if let Some(ButtonPress { button_type: ButtonType::Shift, .. }) = global_modifier {
                                    channel.phrase_mut(index).clear_events();
                                } else {
                                    surface.show_phrase(surface.channel_shown(), index);
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

    // Draw APC specific things
    fn draw(&mut self, sequencer: &mut Sequencer, surface: &mut Surface) {
        match surface.view {
            View::Channel => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Draw main grid
                let events = loopable.events().iter();
                self.draw_loopable_events(events, surface.phrase_offset(surface.channel_shown()), 0, self.loopable_ticks_in_grid(surface), Self::HEAD_COLOR, Self::TAIL_COLOR);

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
    }
}
