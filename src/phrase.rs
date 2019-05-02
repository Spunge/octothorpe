
use std::ops::Range;

use super::bars_to_ticks;
use super::pattern::Pattern;
use super::note::NoteOff;
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::playable::Playable;
use super::message::Message;

#[derive(Clone)]
pub struct PlayedPattern {
    pub index: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

pub struct Phrase {
    pub playable: Playable,
    pub played_patterns: Vec<PlayedPattern>,
}

impl Phrase {
    fn create(played_patterns: Vec<PlayedPattern>) -> Self {
        Phrase { playable: Playable::new(4, 4), played_patterns, }
    }

    pub fn new() -> Self {
        Phrase::create(vec![])
    }
    
    pub fn default() -> Self {
        Phrase::create(vec![
            PlayedPattern { index: 0, start: bars_to_ticks(0), end: bars_to_ticks(4) },
            //PlayedPattern { index: 0, start: bars_to_ticks(1), end: bars_to_ticks(2) },
            //PlayedPattern { index: 0, start: bars_to_ticks(2), end: bars_to_ticks(3) },
            //PlayedPattern { index: 0, start: bars_to_ticks(3), end: bars_to_ticks(4) },
        ])
    }

    pub fn toggle_pattern(&mut self, x: Range<u8>, index: u8) -> Vec<Message> {
        let start = self.playable.ticks_offset() + self.playable.ticks_per_led() * x.start as u32;
        let end = self.playable.ticks_offset() + self.playable.ticks_per_led() * (x.end + 1) as u32;

        let patterns = self.played_patterns.len();
        
        self.played_patterns.retain(|played_pattern| {
            (played_pattern.start < start || played_pattern.start >= end) || played_pattern.index != index as usize
        });

        if patterns == self.played_patterns.len() || x.start != x.end {
            self.played_patterns.push(PlayedPattern { index: index as usize, start, end });
        }

        let mut messages = self.playable.main_grid.clear(false);
        messages.extend(self.draw_phrase());
        messages
    }
   
    pub fn draw_phrase(&mut self) -> Vec<Message> {
        let played_pattern_coords = self.played_patterns.iter()
            .map(|pattern| {
                (pattern.start, pattern.end, pattern.index as i32)
            })
            .collect();

        self.playable.try_switch_coords(played_pattern_coords)
    }

    pub fn draw(&mut self) -> Vec<Message> {
        vec![ 
            self.draw_phrase(),
            self.playable.draw_length(),
            self.playable.draw_zoom() 
        ].into_iter().flatten().collect()
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        vec![ 
            self.playable.main_grid.clear(force), 
            self.playable.length_grid.clear(force),
            self.playable.zoom_grid.clear(force) 
        ].into_iter().flatten().collect()
    }

    pub fn playing_notes(&self, cycle: &Cycle, patterns: &[Pattern]) -> Vec<(TimedMessage, NoteOff)> {
        self.played_patterns.iter()
            .filter_map(|played_pattern| {
                let cycle_start = cycle.start % self.playable.ticks;
                let cycle_end = cycle_start + cycle.ticks;

                // Is pattern playing?
                if played_pattern.start < cycle_end && played_pattern.end > cycle_start {
                    let played_pattern_length = (played_pattern.end - played_pattern.start);
                    let pattern_length = patterns[played_pattern.index].playable.ticks;
                    // Dirty way to round up
                    let loops = (played_pattern_length + pattern_length - 1) / pattern_length;

                    let notes = (0..loops).flat_map(move |iteration| {
                        patterns[played_pattern.index].notes.iter()
                            .filter_map(move |note| {
                                let note_start = note.start + played_pattern.start + iteration * pattern_length;

                                // Does note fall in cycle?
                                if note_start >= cycle_start && note_start < cycle_end {
                                    let delta_ticks = note_start - cycle_start;
                                    let delta_frames = (delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;

                                    let message = TimedMessage::new(delta_frames, note.message());
                                    let note_off = note.note_off(cycle.absolute_start + delta_ticks);

                                    Some((message, note_off ))
                                } else {
                                    None
                                }
                            })
                    });

                    
                    Some(notes)
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}
