
use super::controller::input::*;

#[derive(Debug, PartialEq)]
pub enum View {
    Track,
    Sequence,
    Timeline,
}

pub struct Surface {
    pub view: View,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    track_shown: u8,
    sequence_shown: u8,
    pub timeline_offset: u32,

    offset_factor: f64,

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
        Surface { 
            view: View::Track, 
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            track_shown: 0,
            sequence_shown: 0,
            timeline_offset: 0,

            offset_factor: 0.0,

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

    pub fn pattern_shown(&self) -> u8 { self.pattern_shown[self.track_shown()] }
    pub fn set_pattern_shown(&mut self, index: u8) { self.pattern_shown[self.track_shown()] = index }
    pub fn pattern_offset(&self, index: usize) -> u32 { self.pattern_offsets[index] }
    pub fn set_pattern_offset(&mut self, index: usize, ticks: u32) { self.pattern_offsets[index] = ticks }
    pub fn shown_pattern_offset(&self) -> u32 { self.pattern_offset(self.track_shown()) }
    pub fn set_shown_pattern_offset(&mut self, offset: u32) { self.set_pattern_offset(self.track_shown(), offset) }
    pub fn pattern_zoom_level(&self) -> u8 { self.pattern_zoom_level }
    pub fn set_pattern_zoom_level(&mut self, level: u8) { self.pattern_zoom_level = level }
    // TODO - We don't use the 60 array indexes that are there
    pub fn pattern_base_note(&self, index: usize) -> u8 { self.pattern_base_notes[index] }
    pub fn shown_pattern_base_note(&self) -> u8 { self.pattern_base_notes[self.track_shown()] }
    pub fn set_shown_pattern_base_note(&mut self, base_note: u8) { 
        if base_note <= 118 && base_note >= 22 { 
            self.pattern_base_notes[self.track_shown()] = base_note ;
        }
    }

    pub fn phrase_shown(&self) -> u8 { self.phrase_shown[self.track_shown()] }
    pub fn set_phrase_shown(&mut self, index: u8) { self.phrase_shown[self.track_shown()] = index }
    pub fn phrase_offset(&self, index: usize) -> u32 { self.phrase_offsets[index] }
    pub fn set_phrase_offset(&mut self, index: usize, ticks: u32) { self.phrase_offsets[index] = ticks }
    pub fn shown_phrase_offset(&self) -> u32 { self.phrase_offset(self.track_shown()) }
    pub fn set_shown_phrase_offset(&mut self, offset: u32) { self.set_phrase_offset(self.track_shown(), offset) }
    pub fn phrase_zoom_level(&self) -> u8 { self.phrase_zoom_level }
    pub fn set_phrase_zoom_level(&mut self, level: u8) { self.phrase_zoom_level = level }

    pub fn set_offset_factor(&mut self, factor: f64) {
        self.offset_factor = factor;
    }

    pub fn get_offset(&self, length: u32, grid_width: u32) -> u32 {
        let max = length - grid_width;
        (self.offset_factor * max as f64) as u32
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

    pub fn last_occurred_event_after<F>(&self, controller_track_offset: u8, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
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
