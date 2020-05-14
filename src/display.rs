
use super::controller::input::*;
use super::sequencer::*;
use super::loopable::*;
use super::track::*;
use super::memory::*;
use super::TickRange;
use std::ops::Range;

pub enum LedColor {
    Green,
    Orange,
    Red,
}
pub struct DisplayParameters {
    offset_x: u32,
    offset_y: u8,
    min_offset_y: u8,
    max_offset_y: u8,
    zoom_level: u8,
    ticks_per_button: u32,
    head_color: LedColor,
    tail_color: LedColor,
}

impl DisplayParameters {
    pub fn new(offset_y: u8, min_offset_y: u8, max_offset_y: u8, ticks_per_button: u32, head_color: LedColor, tail_color: LedColor) -> Self {
        Self { 
            offset_x: 0, 
            offset_y,
            min_offset_y,
            max_offset_y,
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
    pub fn adjust_offset_y(&self, offset: u8) -> u8 { 
        if offset > self.max_offset_y { 
            self.max_offset_y 
        } else if offset < self.min_offset_y {
            self.min_offset_y 
        } else { offset }
    }

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

pub trait Display {
    type Loopable: Loopable;
    fn new(button_ticks: u32) -> Self;
    fn parameters(&self) -> &DisplayParameters;
    fn parameters_mut(&mut self) -> &mut DisplayParameters;
    fn loopable_mut<'a>(&self, track: &'a mut Track, track_index: usize) -> &'a mut Self::Loopable;
    fn loopable<'a>(&self, track: &'a Track, track_index: usize) -> &'a Self::Loopable;

    fn process_inputevent(&mut self, event: &InputEvent, grid_x_range: Range<u8>, grid_offset_x: u8, memory: &mut Memory, track: &mut Track, track_index: usize) {
        match event.event_type {
            InputEventType::ButtonPressed(button_type) => {
                let modifier = memory.buttons.modifier(button_type);

                match button_type {
                    // TODO - button offset & width
                    ButtonType::Grid(x, y) => {
                        let ticks_per_button = self.parameters().ticks_per_button();
                        let start = x as u32 * ticks_per_button + self.parameters().offset_x();
                        // Get range of only the button we're pressing
                        let mut tick_range = TickRange::new(start, start + ticks_per_button);
                        let y = self.parameters().offset_y() + y;
                        let loopable = self.loopable_mut(track, track_index);

                        // Are we trying to click an existing event that we want to delete?
                        if let (None, true) = (modifier, loopable.contains_events_starting_in(tick_range, y)) {
                            loopable.remove_events_starting_in(tick_range, y);
                        } else {
                            // Add event get x from modifier when its a grid button in the same row
                            if let Some(ButtonType::Grid(mod_x, mod_y)) = modifier {
                                if *mod_y == y { 
                                    tick_range.start = *mod_x as u32 * ticks_per_button + self.parameters().offset_x();
                                }
                            }

                            // TODO - Note velocity
                            loopable.add_default_event(tick_range, y);
                        }
                    },
                    _ => (),
                }

            },
            _ => (),
        }
    }
}

impl Display for PatternDisplay {
    type Loopable = Pattern;
    fn new(button_ticks: u32) -> Self {
        Self {
            parameters: DisplayParameters::new(58, 22, 118, button_ticks, LedColor::Green, LedColor::Orange),
            shown: [0; 16],
        }
    }
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }
    fn loopable_mut<'a>(&self, track: &'a mut Track, track_index: usize) -> &'a mut Self::Loopable { 
        track.pattern_mut(self.shown[track_index])
    }
    fn loopable<'a>(&self, track: &'a Track, track_index: usize) -> &'a Self::Loopable {
        track.pattern(self.shown[track_index])
    }
}
impl PatternDisplay {
    pub fn shown_pattern(&self, track_index: usize) -> u8 { self.shown[track_index] }
}
impl Display for PhraseDisplay {
    type Loopable = Phrase;
    fn new(button_ticks: u32) -> Self {
        Self {
            parameters: DisplayParameters::new(0, 0, 4, button_ticks, LedColor::Red, LedColor::Orange),
            shown: [0; 16],
        }
    }
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }
    fn loopable_mut<'a>(&self, track: &'a mut Track, track_index: usize) -> &'a mut Self::Loopable { 
        track.phrase_mut(self.shown[track_index])
    }
    fn loopable<'a>(&self, track: &'a Track, track_index: usize) -> &'a Self::Loopable {
        track.phrase(self.shown[track_index])
    }
}
impl PhraseDisplay {
    pub fn shown_phrase(&self, track_index: usize) -> u8 { self.shown[track_index] }
}
impl Display for TimelineDisplay {
    type Loopable = Timeline;
    fn new(button_ticks: u32) -> Self {
        Self {
            parameters: DisplayParameters::new(0, 0, 4, button_ticks, LedColor::Red, LedColor::Green),
        }
    }
    fn parameters(&self) -> &DisplayParameters { &self.parameters }
    fn parameters_mut(&mut self) -> &mut DisplayParameters { &mut self.parameters }
    fn loopable_mut<'a>(&self, track: &'a mut Track, _track_index: usize) -> &'a mut Self::Loopable { 
        track.timeline_mut()
    }
    fn loopable<'a>(&self, track: &'a Track, track_index: usize) -> &'a Self::Loopable {
        track.timeline()
    }
}

