
use std::ops::Range;

use super::bars_to_ticks;
use super::pattern::Pattern;
use super::cycle::Cycle;
use super::playable::Playable;
use super::message::Message;

#[derive(Debug, Clone)]
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

    pub fn reset(&mut self) {
        self.played_patterns = vec![];
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
   
    pub fn playing_patterns(&self, cycle: &Cycle, patterns: &[Pattern]) -> Vec<PlayedPattern> {
        // Fill up patterns that are larger as 1 iterationn of pattern with multiple playedpatterns
        // of the same kind
        self.played_patterns.iter()
            .flat_map(|played_pattern| {
                let played_pattern_length = played_pattern.end - played_pattern.start;
                let pattern_length = patterns[played_pattern.index].playable.ticks;
                // Dirty way to round up
                let iterations = (played_pattern_length + pattern_length - 1) / pattern_length;

                (0..iterations).map(move |iteration| {
                    let start = played_pattern.start + iteration * pattern_length;
                    let mut end = start + pattern_length;
                    if played_pattern.end < end {
                        end = played_pattern.end;
                    }

                    PlayedPattern { start, end, index: played_pattern.index }
                })
            })
            .filter_map(|mut played_pattern| {
                let plays = cycle.start / self.playable.ticks;
                let cycle_start = cycle.start % self.playable.ticks;
                let cycle_end = cycle_start + cycle.ticks;
                // Is pattern playing?
                if played_pattern.start < cycle_end && played_pattern.end > cycle_start {
                    // Move played pattern to current cycle so we don't need phrase to compare
                    // notes
                    played_pattern.start += plays * self.playable.ticks;
                    played_pattern.end += plays * self.playable.ticks;

                    Some(played_pattern)
                } else {
                    None
                }
            })
            .collect()
    }
}
