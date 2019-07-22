
use std::ops::Range;

use super::pattern::Pattern;
use super::cycle::Cycle;
use super::playable::{Playable, Drawable};
use super::handlers::TimebaseHandler;

#[derive(Debug, Clone)]
pub struct PlayedPattern {
    pub index: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

impl Playable for Phrase {
    // 4 beats of 1920 ticks
    const MINIMUM_LENGTH: u32 = 4 * TimebaseHandler::TICKS_PER_BEAT;
}
impl Drawable for Phrase {
    // led states for head & tail
    const HEAD: u8 = 3;
    const TAIL: u8 = 5;
}

pub struct Phrase {
    // Length in ticks
    pub length: u32,
    pub zoom: u32,
    pub offset: u32,

    pub played_patterns: Vec<PlayedPattern>,
}

impl Phrase {
    fn create(played_patterns: Vec<PlayedPattern>, length: u32) -> Self {
        Phrase { length, played_patterns, zoom: 1, offset: 0 }
    }

    pub fn new() -> Self {
        Phrase::create(vec![], Phrase::MINIMUM_LENGTH)
    }
    
    pub fn default() -> Self {
        let played_patterns = vec![
            PlayedPattern { index: 0, start: Self::bars_to_ticks(0), end: Self::bars_to_ticks(2) },
            PlayedPattern { index: 1, start: Self::bars_to_ticks(2), end: Self::bars_to_ticks(4) },
            //PlayedPattern { index: 0, start: self.bars_to_ticks(2), end: self.bars_to_ticks(3) },
            //PlayedPattern { index: 0, start: self.bars_to_ticks(3), end: self.bars_to_ticks(4) },
        ];

        Phrase::create(played_patterns, Phrase::MINIMUM_LENGTH)
    }

    pub fn led_states(&mut self) -> Vec<(i32, i32, u8)> {
        let coords = self.played_patterns.iter()
            .map(|pattern| {
                (pattern.start, pattern.end, pattern.index as i32)
            })
            .collect();

        self.led_states(coords)
    }

    pub fn toggle_pattern(&mut self, x: Range<u8>, index: u8) {
        let start = self.ticks_offset() + self.ticks_per_led() * x.start as u32;
        let end = self.ticks_offset() + self.ticks_per_led() * (x.end + 1) as u32;

        let patterns = self.played_patterns.len();
        
        // Shorten pattern when a button is clicked that falls in the range of the note
        for play in &mut self.played_patterns {
            if play.start < start && play.end > start && play.index == index as usize {
                play.end = start;
            }
        }

        self.played_patterns.retain(|play| {
            (play.start < start || play.start >= end) || play.index != index as usize
        });

        if patterns == self.played_patterns.len() || x.start != x.end {
            self.played_patterns.push(PlayedPattern { index: index as usize, start, end });
        }
    }
   
    pub fn playing_patterns(&self, cycle: &Cycle, sequence_ticks: u32, patterns: &[Pattern]) -> Vec<PlayedPattern> {
        // Fill up patterns that are larger as 1 iterationn of pattern with multiple playedpatterns
        // of the same kind
        self.played_patterns.iter()
            .flat_map(|played_pattern| {
                let played_pattern_length = played_pattern.end - played_pattern.start;
                let pattern_length = patterns[played_pattern.index].length();
                // Dirty way to round up
                let iterations = (played_pattern_length + pattern_length - 1) / pattern_length;

                (0..iterations).map(move |iteration| {
                    let start = played_pattern.start + iteration * pattern_length;
                    let mut end = start + pattern_length;
                    // It could be pattern is cut short as played pattern is not exactly divisible
                    // by pattern length
                    if played_pattern.end < end {
                        end = played_pattern.end;
                    }

                    // Return played pattern for this iteration through phrase & pattern for next
                    // iteration through phrase
                    PlayedPattern { 
                        // As it could be this is next sequence, add sequence ticks offset
                        start: start + sequence_ticks,
                        end: end + sequence_ticks,
                        index: played_pattern.index 
                    }
                })
            })
            .filter_map(|mut played_pattern| {
                let plays = cycle.start / self.length();
                let cycle_start = cycle.start % self.length();
                let cycle_end = cycle_start + cycle.ticks;

                // Is pattern playing?
                if played_pattern.start < cycle_end && played_pattern.end > cycle_start {
                    // Move played pattern to current cycle so we don't need phrase to compare
                    // notes
                    played_pattern.start += plays * self.length();
                    played_pattern.end += plays * self.length();

                    Some(played_pattern)
                } else {
                    None
                }
            })
            .collect()
    }
}
