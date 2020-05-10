
use super::controller::input::*;
use super::TimebaseHandler;
use super::Sequencer;
use super::loopable::*;
use super::events::*;

#[derive(Debug, PartialEq)]
pub enum View {
    Track,
    Sequence,
    Timeline,
}

pub enum TrackView {
    Split,
    Pattern,
    Phrase,
    Timeline,
}

pub enum LoopableType {
    Timeline,
    Phrase { shown: [u8; 16] },
    Pattern { shown: [u8; 16] },
}
pub struct LoopableGrid {
    offset_x: u32,
    offset_y: u8,
    zoom_level: u8,
    ticks_per_button: u32,
    pub loopable_type: LoopableType,
}

impl LoopableGrid {
    pub fn new(loopable_type: LoopableType, offset_y: u8, ticks_per_button: u32) -> Self {
        Self { offset_x: 0, offset_y, zoom_level: 4, ticks_per_button, loopable_type }
    }

    pub fn ticks_per_button(&self) -> u32 {
        self.ticks_per_button / self.zoom_level as u32
    }
    pub fn ticks_in_grid(&self, grid_width: u32) -> u32 {
        self.ticks_per_button() * grid_width
    }

    pub fn max_offset_x(&mut self, length: u32, grid_width: u8) -> u32 {
        let ticks_in_grid = self.ticks_per_button() * grid_width as u32;
        if ticks_in_grid < length { length - ticks_in_grid } else { 0 }
    }

    pub fn set_offset_x(&mut self, ticks: u32, max: u32) { 
        let adjusted_offset = (ticks / self.ticks_per_button()) * self.ticks_per_button();
        self.offset_x = if adjusted_offset < max { adjusted_offset } else { max };
    }
    pub fn offset_x(&self) -> u32 { self.offset_x }

    pub fn set_offset_y(&mut self, offset: u8) { 
        let offset = match self.loopable_type {
            LoopableType::Pattern { .. } => {
                if offset > 118 { 118 } else if offset < 22 { 22 } else { offset }
            },
            // Phrases & timeline don't support scrolling (yet)
            _ => if offset > 4 { 4 } else { offset }
        };
        self.offset_y = offset;
    }
    pub fn offset_y(&self) -> u8 { self.offset_y }

    pub fn zoom_level(&self) -> u8 { self.zoom_level }
    pub fn set_zoom_level(&mut self, level: u8) {
        // Why not support 7?
        if level != 7 {
            self.zoom_level = level;
        }
    }
}


pub struct Surface {
    pub view: View,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    pub pattern_grid: LoopableGrid,
    pub phrase_grid: LoopableGrid,
    pub timeline_grid: LoopableGrid,

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

    pub fn new() -> Self {
        let pattern_ticks_per_button = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
        let phrase_ticks_per_button = pattern_ticks_per_button * 4;
        let timeline_ticks_per_button = phrase_ticks_per_button * 4;

        Surface { 
            view: View::Track, 
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            pattern_grid: LoopableGrid::new(LoopableType::Pattern { shown: [0; 16] }, 58, pattern_ticks_per_button),
            phrase_grid: LoopableGrid::new(LoopableType::Phrase { shown: [0; 16] }, 0, phrase_ticks_per_button),
            timeline_grid: LoopableGrid::new(LoopableType::Timeline, 0, timeline_ticks_per_button),

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

    pub fn pattern_ticks_per_button(&self) -> u32 { self.pattern_grid.ticks_per_button() }
    pub fn pattern_ticks_in_grid(&self) -> u32 { self.pattern_ticks_per_button() * 8 }
    pub fn phrase_ticks_per_button(&self) -> u32 { self.phrase_grid.ticks_per_button() }
    pub fn phrase_ticks_in_grid(&self) -> u32 { self.phrase_ticks_per_button() * 8 }
    pub fn timeline_ticks_in_grid(&self) -> u32 { self.timeline_grid.ticks_per_button() }

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
