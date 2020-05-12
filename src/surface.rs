
use super::controller::*;
use super::controller::input::*;
use super::TimebaseHandler;
use super::Sequencer;
use super::loopable::*;
use super::events::*;
use super::cycle::*;
use super::mixer::*;

const LENGTH_INDICATOR_USECS: u64 = 200000;

#[derive(Debug, PartialEq)]
pub enum View {
    Track,
    Sequence,
}

pub enum TrackView {
    Split,
    Pattern,
    Phrase,
    Timeline,
}

pub struct DisplayParameters {
    offset_x: u32,
    offset_y: u8,
    zoom_level: u8,
    ticks_per_button: u32,
    head_color: LedColor,
    tail_color: LedColor,
}

pub enum LedColor {
    Green,
    Orange,
    Red,
}

impl DisplayParameters {
    pub fn new(offset_y: u8, ticks_per_button: u32, head_color: LedColor, tail_color: LedColor) -> Self {
        Self { 
            offset_x: 0, 
            offset_y,
            zoom_level: 4,
            ticks_per_button,
            head_color,
            tail_color,
        }
    }
    pub fn ticks_per_button(&self) -> u32 { self.ticks_per_button / self.zoom_level as u32 }
    pub fn ticks_in_grid(&self, grid_width: u8) -> u32 { self.ticks_per_button() * grid_width as u32 }

    pub fn zoom_level(&self) -> u8 { self.zoom_level }
    pub fn set_zoom_level(&mut self, level: u8) {
        if level != 7 { self.zoom_level = level };
    }

    pub fn offset_x(&self) -> u32 { self.offset_x }
    pub fn max_offset_x(&self, loopable_length: u32, grid_width: u8) -> u32 {
        let ticks_in_grid = self.ticks_per_button() * grid_width as u32;
        if ticks_in_grid < loopable_length { loopable_length - ticks_in_grid } else { 0 }
    }
    pub fn adjust_offset_x(&self, ticks: i32) -> i32 {
        (ticks / self.ticks_per_button() as i32) * self.ticks_per_button() as i32
    }
    pub fn set_offset_x(&mut self, ticks: i32, loopable_length: u32, grid_width: u8) {
        let max_offset = self.max_offset_x(loopable_length, grid_width);
        let offset = self.adjust_offset_x(ticks);

        if offset >= 0 && ticks <= max_offset as i32 {
            self.offset_x = offset as u32;
        }
    }

    pub fn offset_y(&self) -> u8 { self.offset_y }

    pub fn head_color(&self) -> &LedColor { &self.head_color }
    pub fn tail_color(&self) -> &LedColor { &self.tail_color }
}

pub struct PatternDisplay {
    parameters: DisplayParameters,
    shown: [u8; 16],
}
pub struct PhraseDisplay {
    parameters: DisplayParameters,
    shown: [u8; 16],
}
pub struct TimelineDisplay {
    parameters: DisplayParameters,
}

pub trait LoopableDisplay {
    fn parameters(&self) -> &DisplayParameters;
    fn parameters_mut(&mut self) -> &mut DisplayParameters;

    fn adjust_offset_y(&self, offset: u8) -> u8 { 
        if offset > 4 { 4 } else { offset }
    }

    fn loopable_length(&self, sequencer: &Sequencer, track_index: usize) -> u32;
}

impl LoopableDisplay for PatternDisplay {
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }

    fn adjust_offset_y(&self, offset: u8) -> u8 { 
        if offset > 118 { 118 } else if offset < 22 { 22 } else { offset }
    }

    fn loopable_length(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        sequencer.track(track_index).pattern(self.shown[track_index]).length()
    }
    //fn loopable_events(&self, sequencer: &Sequencer, track_index: usize) -> &Vec<dyn LoopableEvent> {
        //&sequencer.track(track_index).pattern(self.shown[track_index]).events()
    //}
}
impl PatternDisplay {
    fn new(offset_y: u8, ticks_per_button: u32) -> Self {
        let parameters = DisplayParameters::new(offset_y, ticks_per_button, LedColor::Green, LedColor::Orange);
        Self { parameters, shown: [0; 16] }
    }
}
impl LoopableDisplay for PhraseDisplay {
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }
    fn loopable_length(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        sequencer.track(track_index).phrase(self.shown[track_index]).length()
    }
    //fn loopable_events(&self, sequencer: &Sequencer, track_index: usize) -> &Vec<LoopableEvent> {
        //&sequencer.track(track_index).phrase(self.shown[track_index]).events()
    //}
}
impl PhraseDisplay {
    fn new(offset_y: u8, ticks_per_button: u32) -> Self {
        let parameters = DisplayParameters::new(offset_y, ticks_per_button, LedColor::Red, LedColor::Orange);
        Self { parameters, shown: [0; 16] }
    }
}
impl LoopableDisplay for TimelineDisplay {
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }
    fn loopable_length(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        sequencer.track(track_index).timeline.length()
    }
    //fn loopable_events(&self, sequencer: &Sequencer, track_index: usize) -> &Vec<dyn LoopableEvent> {
        //&sequencer.track(track_index).timeline.events()
    //}
}
impl TimelineDisplay {
    fn new(offset_y: u8, ticks_per_button: u32) -> Self {
        let parameters = DisplayParameters::new(offset_y, ticks_per_button, LedColor::Green, LedColor::Red);
        Self { parameters }
    }
}

pub struct Surface {
    pub view: View,
    pub track_view: TrackView,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    pub pattern_display: PatternDisplay,
    pub phrase_display: PhraseDisplay,
    pub timeline_display: TimelineDisplay,

    track_shown: u8,
    sequence_shown: u8,

    phrase_shown: [u8; 16],
    phrase_zoom_level: u8,
    phrase_offsets: [u32; 16],

    pattern_shown: [u8; 16],
    pattern_zoom_level: u8,
    pattern_offsets: [u32; 16],
    pattern_base_notes: [u8; 16],

}

impl Surface {
    pub fn new(client: &jack::Client) -> Self {
        let pattern_ticks_per_button = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
        let phrase_ticks_per_button = pattern_ticks_per_button * 4;
        let timeline_ticks_per_button = phrase_ticks_per_button * 4;

        Surface { 
            view: View::Track, 
            track_view: TrackView::Split,
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            pattern_display: PatternDisplay::new(58, pattern_ticks_per_button),
            phrase_display: PhraseDisplay::new(0, phrase_ticks_per_button),
            timeline_display: TimelineDisplay::new(0, timeline_ticks_per_button),

            track_shown: 0,
            sequence_shown: 0,

            phrase_shown: [0; 16],
            phrase_zoom_level: 4,
            phrase_offsets: [0; 16],

            pattern_shown: [0; 16],
            pattern_zoom_level: 4,
            pattern_offsets: [0; 16],
            pattern_base_notes: [60; 16],
        }
    }

    // Get grid offset, width & loopable display so we know what to draw where
    pub fn loopable_display_mut(&mut self, controller_offset_x: u8) -> (u8, u8, Box<&mut dyn LoopableDisplay>) {
        match self.track_view {
            TrackView::Split => {
                if(controller_offset_x >= 8) {
                    (0, 8, Box::new(&mut self.pattern_display))
                } else {
                    (0, 8, Box::new(&mut self.phrase_display))
                }
            },
            TrackView::Pattern => {
                (controller_offset_x, 16, Box::new(&mut self.pattern_display))
            },
            TrackView::Phrase => {
                (controller_offset_x, 16, Box::new(&mut self.phrase_display))
            },
            TrackView::Timeline => {
                (controller_offset_x, 16, Box::new(&mut self.timeline_display))
            }
        }
    }

    /*
    pub fn process_midi_input(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, mixer: &mut Mixer) {
        // Process identification and other controller specific stuff
        for controller in self.controllers.iter_mut() {
            if ! controller.is_ready {
                controller.identify(&cycle);
            } else {
                // Controller is ready, process input
                for event in controller.input_events(cycle.scope) {
                    // Only process channel note messages
                    match event.event_type {
                        InputEventType::FaderMoved { value, fader_type: FaderType::Track(index) } => {
                            mixer.fader_adjusted(event.time, controller.offset_x + index, value);
                        },
                        InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                            // Cueknow is only used in track views
                            if let View::Track = self.view {
                                // Check if cueknob should respond immediately
                                let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                                let is_first_turn = self.event_memory
                                    .last_occurred_controller_event_after(controller.offset_x, &[InputEvent::is_cue_knob], usecs)
                                    .is_none();

                                let delta_buttons = controller.cue_knob.process_turn(value, is_first_turn);
                                
                                let (grid_offset_x, grid_width, loopable_display) = self.loopable_display_mut(controller.offset_x);
                            }


                            match self.view {
                                View::Track => {
                                    let track = sequencer.track(self.track_shown());
                                    let loopable = track.phrase(self.phrase_shown(self.track_shown()));
                                    let delta_ticks = delta_buttons as i32 * self.phrase_grid.ticks_per_button() as i32;
                                    let new_offset = self.phrase_grid.offset_x() as i32 + delta_ticks;
                                    let offset = if new_offset < 0 { 0 } else { new_offset as u32 };

                                    let max_offset_x = self.phrase_grid.max_offset_x(loopable.length(), 8);
                                    self.phrase_grid.set_offset_x(offset, max_offset_x);
                                },
                                View::Timeline => {
                                    let new_offset = self.timeline_grid.offset_x() as i32 + (delta_buttons as i32 * self.timeline_grid.ticks_per_button() as i32);

                                    if new_offset >= 0 {
                                        let max_offset_x = self.timeline_grid.max_offset_x(sequencer.timeline_end(), 8);
                                        self.timeline_grid.set_offset_x(max_offset_x, new_offset as u32);
                                    }
                                },
                                _ => (),
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
        self.apc20.process_midi_input(&cycle);

        for event in self.apc20.input_events(cycle.scope) {
            // Only process channel note messages
            match event.event_type {
                InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                    // Check if cueknob should respond immediately
                    let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                    let is_first_turn = self.event_memory
                        .last_occurred_controller_event_after(self.apc20.button_offset_x(), &[InputEvent::is_cue_knob], usecs)
                        .is_none();

                    let delta_buttons = self.apc20.cue_knob().process_turn(value, is_first_turn);

                    match self.view {
                        View::Track => {
                            let track = sequencer.track(self.track_shown());
                            let loopable = track.phrase(self.phrase_shown(self.track_shown()));
                            let delta_ticks = delta_buttons as i32 * self.phrase_grid.ticks_per_button() as i32;
                            let new_offset = self.phrase_grid.offset_x() as i32 + delta_ticks;
                            let offset = if new_offset < 0 { 0 } else { new_offset as u32 };

                            let max_offset_x = self.phrase_grid.max_offset_x(loopable.length(), 8);
                            self.phrase_grid.set_offset_x(offset, max_offset_x);
                        },
                        View::Timeline => {
                            let new_offset = self.timeline_grid.offset_x() as i32 + (delta_buttons as i32 * self.timeline_grid.ticks_per_button() as i32);

                            if new_offset >= 0 {
                                let max_offset_x = self.timeline_grid.max_offset_x(sequencer.timeline_end(), 8);
                                self.timeline_grid.set_offset_x(max_offset_x, new_offset as u32);
                            }
                        },
                        _ => (),
                    }
                },
                _ => (),
            };
        };

        for event in self.input_events(cycle.scope) {
            // Only process channel note messages
            match event.event_type {
                // TODO - Shift events in loopable to right/left when holding shift
                InputEventType::KnobTurned { value, knob_type: KnobType::Cue } => {
                    // Check if cueknob should respond immediately
                    let usecs = cycle.time_at_frame(event.time) - LENGTH_INDICATOR_USECS;
                    let is_first_turn = surface.event_memory
                        .last_occurred_controller_event_after(Self::TRACK_OFFSET, &[InputEvent::is_cue_knob], usecs)
                        .is_none();

                    let delta_buttons = self.cue_knob().process_turn(value, is_first_turn);

                    match surface.view {
                        View::Track => {
                            let loopable = self.shown_loopable(sequencer, surface);
                            let loopable_grid = self.loopable_grid_mut(surface);
                            let delta_ticks = delta_buttons as i32 * loopable_grid.ticks_per_button() as i32;
                            let new_offset = loopable_grid.offset_x() as i32 + delta_ticks;
                            let offset = if new_offset < 0 { 0 } else { new_offset as u32 };

                            let max_offset_x = loopable_grid.max_offset_x(loopable.length(), 8);
                            loopable_grid.set_offset_x(offset, max_offset_x);
                        },
                        View::Timeline => {
                            let new_offset = surface.timeline_grid.offset_x() as i32 + (delta_buttons as i32 * surface.timeline_grid.ticks_per_button() as i32);

                            if new_offset >= 0 {
                                let max_offset_x = surface.timeline_grid.max_offset_x(sequencer.timeline_end(), 8);
                                surface.timeline_grid.set_offset_x(max_offset_x, new_offset as u32);
                            }
                        },
                        _ => (),
                    }
                },
                InputEventType::ButtonPressed(button_type) => {
                    // Register press in memory to keep track of modifing buttons
                    surface.button_memory.press(Self::TRACK_OFFSET, button_type);
                    let global_modifier = surface.button_memory.global_modifier(button_type);

                    // Do the right thing in the right visualization
                    match surface.view {
                        View::Track => {
                            match button_type {
                                ButtonType::Solo(index) => {
                                    // We divide by zoom level, so don't start at 0
                                    let zoom_level = index + 1;
                                    self.loopable_grid_mut(surface).set_zoom_level(zoom_level);
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
                                ButtonType::Activator(track) => {
                                    sequence.toggle_active((track + Self::TRACK_OFFSET) as usize)
                                },
                                _ => (),
                            }
                        },
                        View::Timeline => {
                            match button_type {
                                ButtonType::Grid(x, y) => {
                                    let track = sequencer.track_mut(surface.track_shown());

                                    // Add track offset to make it possible to draw across multiple controllers
                                    let start = (Self::TRACK_OFFSET + x) as u32 * surface.timeline_grid.ticks_per_button() + surface.timeline_grid.offset_x();
                                    let mut tick_range = TickRange::new(start, start + surface.timeline_grid.ticks_per_button());

                                    // Should we delete the event we're clicking?
                                    if let (None, true) = (global_modifier, track.timeline.contains_events_starting_in(tick_range, y)) {
                                        track.timeline.remove_events_starting_in(tick_range, y);
                                    } else {
                                        // Add event get x from modifier when its a grid button in the same row
                                        if let Some(ButtonPress { button_type: ButtonType::Grid(mod_x, mod_y), controller_track_offset }) = global_modifier {
                                            if *mod_y == y { 
                                                // Add track offset off modifier to make it possible to draw across controllers
                                                tick_range.start = (mod_x + controller_track_offset) as u32 * surface.timeline_grid.ticks_per_button() + surface.timeline_grid.offset_x();
                                            }
                                        }

                                        // Switch start & stop if stop button was pressed before start button
                                        if tick_range.start > tick_range.stop {
                                            let start = tick_range.start;
                                            // As stop is @ end of button & start is @ start of
                                            // button, offset start & stop by a button
                                            tick_range.start = tick_range.stop - surface.timeline_grid.ticks_per_button();
                                            tick_range.stop = start + surface.timeline_grid.ticks_per_button();
                                        }

                                        track.timeline.add_complete_event(LoopablePhraseEvent::new(tick_range.start, tick_range.stop, y));
                                    }
                                },
                                _ => (),
                            }
                        }
                    }

                    // Independent of current view
                    match button_type {
                        ButtonType::Track(index) => {
                            match surface.view {
                                View::Track | View::Timeline => {
                                    if surface.track_shown() == index as usize {
                                        let view = if matches!(surface.view, View::Timeline) { View::Track } else { View::Timeline };
                                        surface.switch_view(view);
                                    } else {
                                        surface.show_track(index + Self::TRACK_OFFSET);
                                    }
                                },
                                _ => {
                                    surface.switch_view(View::Timeline);
                                    surface.show_track(index + Self::TRACK_OFFSET);
                                },
                            }
                        },
                        // Switch to timeline, when timeline already shown, switch to track
                        ButtonType::Master => {
                            let view = match surface.view {
                                View::Sequence => View::Track,
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

            // Keep track of event so we can use it to calculate double presses etc.
            surface.event_memory.register_event(Self::TRACK_OFFSET, cycle.time_at_frame(event.time), event.event_type);
        }



        //self.apc40.process_midi_input(&cycle, sequencer, self, mixer);
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer) {
        //self.apc20.output_midi(&cycle, sequencer, self);
        //self.apc40.output_midi(&cycle, sequencer, self);
    }
    */

    pub fn switch_view(&mut self, view: View) { 
        self.view = view;
    }

    pub fn show_track(&mut self, index: u8) { self.track_shown = index; }
    pub fn track_shown(&self) -> usize { self.track_shown as usize }
    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }
    pub fn phrase_shown(&self, track_index: usize) -> u8 { self.phrase_shown[track_index] }
    pub fn show_phrase(&mut self, track_index: usize, index: u8) { self.phrase_shown[track_index] = index }
    pub fn pattern_shown(&self, track_index: usize) -> u8 { self.pattern_shown[track_index] }
    pub fn show_pattern(&mut self, track_index: usize, index: u8) { self.pattern_shown[track_index] = index }

    pub fn set_offsets_by_factor(&mut self, sequencer: &Sequencer, track_index: usize, factor: f64) {
        //let max_phrase_offset = self.max_phrase_offset(sequencer, track_index);
        //let phrase_offset = (max_phrase_offset as f64 * factor) as u32;
        //self.set_phrase_offset(sequencer, track_index, phrase_offset);
        //let max_pattern_offset = self.max_pattern_offset(sequencer, track_index);
        //let pattern_offset = (max_pattern_offset as f64 * factor) as u32;
        //self.set_pattern_offset(sequencer, track_index, pattern_offset);
        //let max_timeline_offset = self.max_timeline_offset(sequencer);
        //let timeline_offset = (max_timeline_offset as f64 * factor) as u32;
        //self.set_timeline_offset(sequencer, timeline_offset);
        // TODO - Timeline
    }
}

#[derive(Debug)]
struct OccurredInputEvent {
    controller_track_offset: u8,
    time: u64,
    event_type: InputEventType,
}

pub struct EventMemory {
    // Remember when the last occurence of input event was for each input event on the controller,
    // this was we can keep track of double clicks or show info based on touched buttons
    occurred_events: Vec<OccurredInputEvent>,
}

impl EventMemory {
    fn new() -> Self {
        Self { occurred_events: vec![] }
    }

    pub fn register_event(&mut self, controller_track_offset: u8, time: u64, event_type: InputEventType) {
        let previous = self.occurred_events.iter_mut()
            .find(|event| event.controller_track_offset == controller_track_offset && event.event_type == event_type);

        if let Some(event) = previous {
            event.time = time;
        } else {
            self.occurred_events.push(OccurredInputEvent { controller_track_offset, time, event_type });
        }
    }

    pub fn last_occurred_global_event_after<F>(&self, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
        self.occurred_events.iter()
            .filter(|event| {
                event.time >= usecs
                    && filters.iter().fold(false, |acc, filter| acc || filter(&event.event_type)) 
            })
            .map(|event| event.time)
            .max()
    }

    pub fn last_occurred_controller_event_after<F>(&self, controller_track_offset: u8, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
        self.occurred_events.iter()
            .filter(|event| {
                controller_track_offset == event.controller_track_offset
                    && event.time >= usecs
                    && filters.iter().fold(false, |acc, filter| acc || filter(&event.event_type)) 
            })
            .map(|event| event.time)
            .max()
    }
}

#[derive(Debug)]
pub struct ButtonPress {
    pub controller_track_offset: u8,
    pub button_type: ButtonType,
}

pub struct ButtonMemory {
    // Remember pressed buttons to provide "modifier" functionality, we *could* use occurred_events
    // for this, but the logic will be a lot easier to understand when we use seperate struct
    pressed_buttons: Vec<ButtonPress>,
}

/*
 * This will keep track of button presses so we can support double press & range press
 */
impl ButtonMemory {
    pub fn new() -> Self {
        Self { pressed_buttons: vec![] }
    }

    //pub fn register_event(&mut self, controller_track_offset: u8, time: u64, InputEvent:)

    // We pressed a button!
    pub fn press(&mut self, controller_track_offset: u8, button_type: ButtonType) {
        // Save pressed_button to keep track of modifing keys (multiple keys pressed twice)
        self.pressed_buttons.push(ButtonPress { controller_track_offset, button_type, });
    }

    pub fn release(&mut self, controller_track_offset: u8, _end: u64, button_type: ButtonType) {
        let pressed_button = self.pressed_buttons.iter().enumerate().rev().find(|(_, pressed_button)| {
            pressed_button.button_type == button_type
                && pressed_button.controller_track_offset == controller_track_offset
        });

        // We only use if let instead of unwrap to not crash when first event is button release
        if let Some((index, _)) = pressed_button {
            self.pressed_buttons.remove(index);
        }
    }

    pub fn modifier(&self, controller_track_offset: u8, button_type: ButtonType) -> Option<ButtonType> {
        self.pressed_buttons.iter()
            .filter(|pressed_button| {
                pressed_button.button_type != button_type
                    && pressed_button.controller_track_offset == controller_track_offset
            })
            .next()
            .and_then(|pressed_button| Some(pressed_button.button_type))
    }

    pub fn global_modifier(&self, button_type: ButtonType) -> Option<&ButtonPress> {
        self.pressed_buttons.iter()
            .filter(|pressed_button| pressed_button.button_type != button_type)
            .next()
            .and_then(|pressed_button| Some(pressed_button))
    }
}
