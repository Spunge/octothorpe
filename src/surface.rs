
use super::controller::input::*;
use super::controller::lights::*;
use super::TimebaseHandler;
use super::Sequencer;
use super::loopable::*;
use super::events::*;

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

pub trait LoopableGrid {
    type Loopable: Loopable;
    const HEAD_COLOR: u8;
    const TAIL_COLOR: u8;
    const TICKS_PER_BUTTON: u32;
    const BUTTONS_IN_GRID: u32;
    fn new() -> Self;
    fn length(&self, sequencer: &Sequencer, track_index: usize) -> u32 { 
        self.shown_loopable(sequencer, track_index).length()
    }
    fn zoom_level(&self) -> u8;
    fn set_zoom_level(&mut self, zoom_level: u8);
    fn ticks_per_button(&self) -> u32 { Self::TICKS_PER_BUTTON / self.zoom_level() as u32 }
    fn ticks_in_grid(&self) -> u32 { self.ticks_per_button() * Self::BUTTONS_IN_GRID }
    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, track_index: usize) -> &'a Self::Loopable;
    fn offset_x(&self) -> u32;
    fn set_offset_x(&mut self, ticks: u32);
    fn offset_y(&self) -> u8;
    fn max_offset(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        let length = self.length(sequencer, track_index);
        if self.ticks_in_grid() < length {
            length - self.ticks_in_grid()
        } else { 0 }
    }

    /*
     * Remove existing events when there's starting events in tick range, otherwise, remove tick
     * range so we can add new event
     */
    fn should_add_event(&self, loopable: &mut impl Loopable, modifier: Option<ButtonType>, x: u8, y: u8) -> Option<TickRange> {
        let row = self.offset_y() + y;

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

    fn grid_button_pressed(&mut self, sequencer: &Sequencer, surface: &surface, controller_x_offset: u8, grid_buttons: u8, button_type: ButtonType) {
    
    }

    fn button_pressed(&mut self, sequencer: &Sequencer, surface: &Surface, controller_x_offset: u8, grid_buttons: u8, button_type: ButtonType) {
        match button_type {
            ButtonType::Grid(x, y) => {
                let modifier = surface.button_memory.modifier(controller_x_offset, button_type);
                let loopable = self.shown_loopable(sequencer, surface.track_shown());

                // We subtract y from 4 as we want lower notes to be lower on
                // the grid, the grid counts from the top
                // We put base note in center of grid

                if let Some(tick_range) = self.should_add_event(loopable, modifier, x, y) {
                    loopable.try_add_starting_event(Self::Loopable::LoopableEvent::new(tick_range.start, self.offset_y() + y, 127));
                    let mut event = loopable.get_last_event_on_row(self.offset_y() + y);
                    event.set_stop(tick_range.stop);
                    event.stop_velocity = Some(127);

                    loopable.add_complete_event(event);
                }
            },
            ButtonType::Solo(index) => {
                // We divide by zoom level, so don't start at 0
                let zoom_level = index + 1;
                if zoom_level != 7 {
                    self.set_zoom_level(zoom_level);
                }
            },
            _ => (),
        }
    }

    fn draw(&self, sequencer: &Sequencer, track_index: usize, controller_x_offset: u8, grid_buttons: u8, lights: &mut Lights) {
        let loopable = self.shown_loopable(sequencer, track_index);

        // Draw zoom grid
        for index in 0 .. self.zoom_level() { lights.solo.draw(index, 1); }

        if loopable.has_explicit_length() {
            for index in 0 .. (loopable.length() / Self::Loopable::minimum_length()) {
                lights.activator.draw(index as u8, 1);
            }
        }

        // Draw main grid
        let grid_height = lights.grid.height();
        loopable.events().iter()
            .filter(|event| (self.offset_y() .. self.offset_y() + grid_height).contains(&event.row()))
            .filter(|event| { 
                let grid_contains_event = event.start() < loopable.length() 
                    && (event.stop().is_none() || event.stop().unwrap() > self.offset_x());

                grid_contains_event || event.is_looping()
            })
            .for_each(|event| {
                // Get buttons from event ticks
                let max_button = lights.grid.width() as i32;
                let start_button = (event.start() as i32 - self.offset_x() as i32) / self.ticks_per_button() as i32;
                let stop_button = if event.stop().is_none() { 
                    start_button + 1
                } else { 
                    // Could be event is to short for 1 button, in that case, draw 1 button
                    // TODO
                    (event.stop().unwrap() as i32 - self.offset_x() as i32) / self.ticks_per_button() as i32
                };

                // Flip grid around to show higher notes higher on the grid (for patterns this does not matter)
                let row = event.row() - self.offset_y();

                // Always draw first button head
                lights.grid.try_draw(start_button, row, Self::HEAD_COLOR);
                // Draw tail depending on wether this is looping note
                let tails = if stop_button >= start_button {
                    vec![(start_button + 1) .. stop_button]
                } else {
                    vec![(start_button + 1) .. max_button, 0 .. stop_button]
                };

                tails.into_iter().for_each(|mut range| {
                    while let Some(x) = range.next() { lights.grid.try_draw(x, row, Self::TAIL_COLOR) }
                })
            });
    }
}

pub struct PatternGrid {
    offset: u32,
    zoom_level: u8,
    pattern_shown: [u8; 16],
    base_note: u8,
}
impl LoopableGrid for PatternGrid {
    type Loopable = Pattern;
    const TICKS_PER_BUTTON: u32 = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
    const BUTTONS_IN_GRID: u32 = 8;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;
    fn new() -> Self { Self { offset: 0, zoom_level: 4, pattern_shown: [0; 16], base_note: 58 } }
    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, zoom_level: u8) { self.zoom_level = zoom_level }
    fn offset_x(&self) -> u32 { self.offset }
    fn set_offset_x(&mut self, ticks: u32) { self.offset = ticks }
    fn offset_y(&self) -> u8 { self.base_note }
    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, track_index: usize) -> &'a Self::Loopable {
        sequencer.track(track_index).pattern(self.pattern_shown(track_index))
    }
}
impl PatternGrid {
    fn pattern_shown(&self, track_index: usize) -> u8 { self.pattern_shown[track_index] }
    fn show_pattern(&mut self, track_index: usize, index: u8) { self.pattern_shown[track_index] = index }
}
pub struct PhraseGrid {
    offset: u32,
    zoom_level: u8,
    phrase_shown: [u8; 16],
}
impl LoopableGrid for PhraseGrid {
    type Loopable = Phrase;
    const TICKS_PER_BUTTON: u32 = PatternGrid::TICKS_PER_BUTTON * 4;
    const BUTTONS_IN_GRID: u32 = 8;
    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;
    fn new() -> Self { Self { offset: 0, zoom_level: 4, phrase_shown: [0; 16] } }
    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, zoom_level: u8) { self.zoom_level = zoom_level }
    fn offset_x(&self) -> u32 { self.offset }
    fn set_offset_x(&mut self, ticks: u32) { self.offset = ticks }
    fn offset_y(&self) -> u8 { 0 }
    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, track_index: usize) -> &'a Self::Loopable {
        sequencer.track(track_index).phrase(self.phrase_shown(track_index))
    }
}
impl PhraseGrid {
    fn phrase_shown(&self, track_index: usize) -> u8 { self.phrase_shown[track_index] }
    fn show_phrase(&mut self, track_index: usize, index: u8) { self.phrase_shown[track_index] = index }
}
pub struct TimelineGrid {
    offset: u32,
    zoom_level: u8,
}
impl LoopableGrid for TimelineGrid {
    type Loopable = Timeline;
    const TICKS_PER_BUTTON: u32 = PhraseGrid::TICKS_PER_BUTTON * 4;
    const BUTTONS_IN_GRID: u32 = 16;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 3;
    fn new() -> Self { Self { offset: 0, zoom_level: 4 } }
    fn length(&self, sequencer: &Sequencer, track_index: usize) -> u32 { 
        let timeline_end = sequencer.get_timeline_end();
        timeline_end + self.ticks_per_button() * 12
    }
    fn zoom_level(&self) -> u8 { self.zoom_level }
    fn set_zoom_level(&mut self, zoom_level: u8) { self.zoom_level = zoom_level }
    fn offset_x(&self) -> u32 { self.offset }
    fn set_offset_x(&mut self, ticks: u32) { self.offset = ticks }
    fn offset_y(&self) -> u8 { 0 }
    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, track_index: usize) -> &'a Self::Loopable {
        &sequencer.track(track_index).timeline
    }
}

pub struct Surface {
    pub view: View,
    pub track_view: TrackView,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    pub pattern_grid: PatternGrid,
    pub phrase_grid: PhraseGrid,
    pub timeline_grid: TimelineGrid,

    track_shown: u8,
    sequence_shown: u8,
    timeline_offset: u32,

    phrase_shown: [u8; 16],
    phrase_zoom_level: u8,
    phrase_offsets: [u32; 16],

    pattern_shown: [u8; 16],
    pattern_zoom_level: u8,
    pattern_offsets: [u32; 16],
    pattern_base_notes: [u8; 16],
}

impl Surface {
    pub const PATTERN_TICKS_PER_BUTTON: u32 = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
    pub const PHRASE_TICKS_PER_BUTTON: u32 = Self::PATTERN_TICKS_PER_BUTTON * 4;
    pub const TIMELINE_TICKS_PER_BUTTON: u32 = Self::PHRASE_TICKS_PER_BUTTON * 1;

    pub fn new() -> Self {
        Surface { 
            view: View::Track, 
            track_view: TrackView::Split,
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            pattern_grid: PatternGrid::new(),
            phrase_grid: PhraseGrid::new(),
            timeline_grid: TimelineGrid::new(),

            track_shown: 0,
            sequence_shown: 0,
            timeline_offset: 0,

            phrase_shown: [0; 16],
            phrase_zoom_level: 4,
            phrase_offsets: [0; 16],

            pattern_shown: [0; 16],
            pattern_zoom_level: 4,
            pattern_offsets: [0; 16],
            pattern_base_notes: [60; 16],
        }
    }

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

    pub fn pattern_ticks_per_button(&self) -> u32 { Self::PATTERN_TICKS_PER_BUTTON / self.pattern_zoom_level() as u32 }
    pub fn pattern_ticks_in_grid(&self) -> u32 { self.pattern_ticks_per_button() * 8 }
    pub fn phrase_ticks_per_button(&self) -> u32 { Self::PHRASE_TICKS_PER_BUTTON / self.phrase_zoom_level() as u32 }
    pub fn phrase_ticks_in_grid(&self) -> u32 { self.phrase_ticks_per_button() * 8 }
    pub fn timeline_ticks_in_grid(&self) -> u32 { Self::TIMELINE_TICKS_PER_BUTTON * 16 }

    pub fn timeline_offset(&self) -> u32 { self.timeline_offset }
    pub fn set_timeline_offset(&mut self, sequencer: &Sequencer, offset: u32) { 
        let max_offset = self.max_timeline_offset(sequencer);
        let adjusted_offset = (offset / Self::TIMELINE_TICKS_PER_BUTTON) * Self::TIMELINE_TICKS_PER_BUTTON;
        self.timeline_offset = if adjusted_offset < max_offset { adjusted_offset } else { max_offset };
    }
    pub fn get_timeline_length(&self, sequencer: &Sequencer) -> u32 {
        let timeline_end = sequencer.get_timeline_end();
        timeline_end + Self::TIMELINE_TICKS_PER_BUTTON * 12
    }
    pub fn max_timeline_offset(&self, sequencer: &Sequencer) -> u32 {
        let timeline_length = self.get_timeline_length(sequencer);
        if self.timeline_ticks_in_grid() < timeline_length {
            timeline_length - self.timeline_ticks_in_grid()
        } else { 0 }
    }

    pub fn pattern_offset(&self, index: usize) -> u32 { self.pattern_offsets[index] }
    pub fn max_pattern_offset(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        let pattern_length = sequencer.track(track_index).pattern(self.pattern_shown(track_index)).length();

        if self.pattern_ticks_in_grid() < pattern_length {
            pattern_length - self.pattern_ticks_in_grid()
        } else { 0 }
    }
    pub fn set_pattern_offset(&mut self, sequencer: &Sequencer, track_index: usize, ticks: u32) {
        let max_offset = self.max_pattern_offset(sequencer, track_index);
        let adjusted_offset = (ticks / self.pattern_ticks_per_button()) * self.pattern_ticks_per_button();
        self.pattern_offsets[track_index] = if adjusted_offset < max_offset { adjusted_offset } else { max_offset };
    }

    pub fn pattern_zoom_level(&self) -> u8 { self.pattern_zoom_level }
    pub fn set_pattern_zoom_level(&mut self, sequencer: &Sequencer, level: u8) { 
        self.pattern_zoom_level = level;
        // - loop shown patterns & adjust offsets so they don't exceed max_offset
        for track_index in 0 .. self.pattern_offsets.len() {
            self.set_pattern_offset(sequencer, track_index, self.pattern_offset(track_index))
        }
    }

    pub fn pattern_base_note(&self, index: usize) -> u8 { self.pattern_base_notes[index] }
    pub fn set_pattern_base_note(&mut self, track_index: usize, base_note: u8) { 
        if base_note <= 118 && base_note >= 22 { 
            self.pattern_base_notes[track_index] = base_note;
        }
    }

    pub fn phrase_offset(&self, track_index: usize) -> u32 { self.phrase_offsets[track_index] }
    pub fn max_phrase_offset(&self, sequencer: &Sequencer, track_index: usize) -> u32 {
        let phrase_length = sequencer.track(track_index).phrase(self.phrase_shown(track_index)).length();

        if self.phrase_ticks_in_grid() < phrase_length {
            phrase_length - self.phrase_ticks_in_grid()
        } else { 0 }
    }
    pub fn set_phrase_offset(&mut self, sequencer: &Sequencer, track_index: usize, ticks: u32) {
        let max_offset = self.max_phrase_offset(sequencer, track_index);
        // Round offset to button
        let adjusted_offset = (ticks / self.phrase_ticks_per_button()) * self.phrase_ticks_per_button();

        // Make sure offset is not > max-offset
        self.phrase_offsets[track_index] = if adjusted_offset < max_offset { adjusted_offset } else { max_offset };
    }

    pub fn phrase_zoom_level(&self) -> u8 { self.phrase_zoom_level }
    pub fn set_phrase_zoom_level(&mut self, sequencer: &Sequencer, level: u8) { 
        self.phrase_zoom_level = level;
        // Loop shown phrases & make sure offset does not exceed max_offset
        for track_index in 0 .. self.phrase_offsets.len() {
            self.set_phrase_offset(sequencer, track_index, self.phrase_offset(track_index))
        }
    }

    pub fn set_offsets_by_factor(&mut self, sequencer: &Sequencer, track_index: usize, factor: f64) {
        let max_phrase_offset = self.max_phrase_offset(sequencer, track_index);
        let phrase_offset = (max_phrase_offset as f64 * factor) as u32;
        self.set_phrase_offset(sequencer, track_index, phrase_offset);
        let max_pattern_offset = self.max_pattern_offset(sequencer, track_index);
        let pattern_offset = (max_pattern_offset as f64 * factor) as u32;
        self.set_pattern_offset(sequencer, track_index, pattern_offset);
        let max_timeline_offset = self.max_timeline_offset(sequencer);
        let timeline_offset = (max_timeline_offset as f64 * factor) as u32;
        self.set_timeline_offset(sequencer, timeline_offset);
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
