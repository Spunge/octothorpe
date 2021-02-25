
use crate::*;
//use super::controller::input::*;
//use super::loopable::*;

#[derive(Debug, PartialEq)]
pub enum View {
    Channel,
    Sequence,
    Timeline,
}

pub struct Surface {
    pub controllers: Vec<Controller>,
    pub view: View,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    channel_shown: u8,
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
            controllers: vec![],

            view: View::Channel, 
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            channel_shown: 0,
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

    pub fn show_channel(&mut self, index: u8) { self.channel_shown = index; }
    pub fn channel_shown(&self) -> usize { self.channel_shown as usize }
    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }
    pub fn phrase_shown(&self, channel_index: usize) -> u8 { self.phrase_shown[channel_index] }
    pub fn show_phrase(&mut self, channel_index: usize, index: u8) { self.phrase_shown[channel_index] = index }
    pub fn pattern_shown(&self, channel_index: usize) -> u8 { self.pattern_shown[channel_index] }
    pub fn show_pattern(&mut self, channel_index: usize, index: u8) { self.pattern_shown[channel_index] = index }

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
    pub fn max_pattern_offset(&self, sequencer: &Sequencer, channel_index: usize) -> u32 {
        let pattern_length = sequencer.channel(channel_index).pattern(self.pattern_shown(channel_index)).length();

        if self.pattern_ticks_in_grid() < pattern_length {
            pattern_length - self.pattern_ticks_in_grid()
        } else { 0 }
    }
    pub fn set_pattern_offset(&mut self, sequencer: &Sequencer, channel_index: usize, ticks: u32) {
        let max_offset = self.max_pattern_offset(sequencer, channel_index);
        let adjusted_offset = (ticks / self.pattern_ticks_per_button()) * self.pattern_ticks_per_button();
        self.pattern_offsets[channel_index] = if adjusted_offset < max_offset { adjusted_offset } else { max_offset };
    }

    pub fn pattern_zoom_level(&self) -> u8 { self.pattern_zoom_level }
    pub fn set_pattern_zoom_level(&mut self, sequencer: &Sequencer, level: u8) { 
        self.pattern_zoom_level = level;
        // - loop shown patterns & adjust offsets so they don't exceed max_offset
        for channel_index in 0 .. self.pattern_offsets.len() {
            self.set_pattern_offset(sequencer, channel_index, self.pattern_offset(channel_index))
        }
    }

    pub fn pattern_base_note(&self, index: usize) -> u8 { self.pattern_base_notes[index] }
    pub fn set_pattern_base_note(&mut self, channel_index: usize, base_note: u8) { 
        if base_note <= 118 && base_note >= 22 { 
            self.pattern_base_notes[channel_index] = base_note;
        }
    }

    pub fn phrase_offset(&self, channel_index: usize) -> u32 { self.phrase_offsets[channel_index] }
    pub fn max_phrase_offset(&self, sequencer: &Sequencer, channel_index: usize) -> u32 {
        let phrase_length = sequencer.channel(channel_index).phrase(self.phrase_shown(channel_index)).length();

        if self.phrase_ticks_in_grid() < phrase_length {
            phrase_length - self.phrase_ticks_in_grid()
        } else { 0 }
    }
    pub fn set_phrase_offset(&mut self, sequencer: &Sequencer, channel_index: usize, ticks: u32) {
        let max_offset = self.max_phrase_offset(sequencer, channel_index);
        // Round offset to button
        let adjusted_offset = (ticks / self.phrase_ticks_per_button()) * self.phrase_ticks_per_button();

        // Make sure offset is not > max-offset
        self.phrase_offsets[channel_index] = if adjusted_offset < max_offset { adjusted_offset } else { max_offset };
    }

    pub fn phrase_zoom_level(&self) -> u8 { self.phrase_zoom_level }
    pub fn set_phrase_zoom_level(&mut self, sequencer: &Sequencer, level: u8) { 
        self.phrase_zoom_level = level;
        // Loop shown phrases & make sure offset does not exceed max_offset
        for channel_index in 0 .. self.phrase_offsets.len() {
            self.set_phrase_offset(sequencer, channel_index, self.phrase_offset(channel_index))
        }
    }

    pub fn set_offsets_by_factor(&mut self, sequencer: &Sequencer, channel_index: usize, factor: f64) {
        let max_phrase_offset = self.max_phrase_offset(sequencer, channel_index);
        let phrase_offset = (max_phrase_offset as f64 * factor) as u32;
        self.set_phrase_offset(sequencer, channel_index, phrase_offset);
        let max_pattern_offset = self.max_pattern_offset(sequencer, channel_index);
        let pattern_offset = (max_pattern_offset as f64 * factor) as u32;
        self.set_pattern_offset(sequencer, channel_index, pattern_offset);
        let max_timeline_offset = self.max_timeline_offset(sequencer);
        let timeline_offset = (max_timeline_offset as f64 * factor) as u32;
        self.set_timeline_offset(sequencer, timeline_offset);
        // TODO - Timeline
    }
}

#[derive(Debug)]
struct OccurredInputEvent {
    controller_channel_offset: u8,
    time: u64,
    event_type: InputEventType,
}

pub struct EventMemory {
    // Remember when the last occurence of input event was for each input event on the controller,
    // this was we can keep channel of double clicks or show info based on touched buttons
    occurred_events: Vec<OccurredInputEvent>,
}

impl EventMemory {
    fn new() -> Self {
        Self { occurred_events: vec![] }
    }

    pub fn register_event(&mut self, controller_channel_offset: u8, time: u64, event_type: InputEventType) {
        let previous = self.occurred_events.iter_mut()
            .find(|event| event.controller_channel_offset == controller_channel_offset && event.event_type == event_type);

        if let Some(event) = previous {
            event.time = time;
        } else {
            self.occurred_events.push(OccurredInputEvent { controller_channel_offset, time, event_type });
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

    pub fn last_occurred_controller_event_after<F>(&self, controller_channel_offset: u8, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
        self.occurred_events.iter()
            .filter(|event| {
                controller_channel_offset == event.controller_channel_offset
                    && event.time >= usecs
                    && filters.iter().fold(false, |acc, filter| acc || filter(&event.event_type)) 
            })
            .map(|event| event.time)
            .max()
    }
}

#[derive(Debug)]
pub struct ButtonPress {
    pub controller_channel_offset: u8,
    pub button_type: ButtonType,
}

pub struct ButtonMemory {
    // Remember pressed buttons to provide "modifier" functionality, we *could* use occurred_events
    // for this, but the logic will be a lot easier to understand when we use seperate struct
    pressed_buttons: Vec<ButtonPress>,
}

/*
 * This will keep channel of button presses so we can support double press & range press
 */
impl ButtonMemory {
    pub fn new() -> Self {
        Self { pressed_buttons: vec![] }
    }

    //pub fn register_event(&mut self, controller_channel_offset: u8, time: u64, InputEvent:)

    // We pressed a button!
    pub fn press(&mut self, controller_channel_offset: u8, button_type: ButtonType) {
        // Save pressed_button to keep channel of modifing keys (multiple keys pressed twice)
        self.pressed_buttons.push(ButtonPress { controller_channel_offset, button_type, });
    }

    pub fn release(&mut self, controller_channel_offset: u8, _end: u64, button_type: ButtonType) {
        let pressed_button = self.pressed_buttons.iter().enumerate().rev().find(|(_, pressed_button)| {
            pressed_button.button_type == button_type
                && pressed_button.controller_channel_offset == controller_channel_offset
        });

        // We only use if let instead of unwrap to not crash when first event is button release
        if let Some((index, _)) = pressed_button {
            self.pressed_buttons.remove(index);
        }
    }

    pub fn modifier(&self, controller_channel_offset: u8, button_type: ButtonType) -> Option<ButtonType> {
        self.pressed_buttons.iter()
            .filter(|pressed_button| {
                pressed_button.button_type != button_type
                    && pressed_button.controller_channel_offset == controller_channel_offset
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
