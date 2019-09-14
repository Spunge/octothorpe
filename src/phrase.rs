
use std::ops::Range;

use super::pattern::{Pattern, PlayedPattern, PlayingPattern};
use super::playable::Playable;
use super::handlers::TimebaseHandler;

#[derive(Debug)]
pub struct PlayingPhrase {
    // Index in sequencers instruments array
    pub instrument: usize,
    // Index in Instruments phrases array
    pub phrase: usize,
    // Start & end of phrases
    pub start: u32,
    pub end: u32,
}

pub struct Phrase {
    pub playable: Playable,
    pub played_patterns: Vec<PlayedPattern>,
}

impl Phrase {
    fn create(played_patterns: Vec<PlayedPattern>) -> Self {
        Phrase { playable: Playable::new(TimebaseHandler::bars_to_ticks(4), TimebaseHandler::bars_to_ticks(4), 3, 5), played_patterns, }
    }

    pub fn new() -> Self {
        Phrase::create(vec![])
    }
    
    pub fn default() -> Self {
        Phrase::create(vec![
            PlayedPattern { index: 0, start: TimebaseHandler::bars_to_ticks(0), end: TimebaseHandler::bars_to_ticks(4) },
            //PlayedPattern { index: 1, start: TimebaseHandler::bars_to_ticks(2), end: TimebaseHandler::bars_to_ticks(4) },
            //PlayedPattern { index: 0, start: TimebaseHandler::bars_to_ticks(2), end: TimebaseHandler::bars_to_ticks(3) },
            //PlayedPattern { index: 0, start: TimebaseHandler::bars_to_ticks(3), end: TimebaseHandler::bars_to_ticks(4) },
        ])
    }

    pub fn led_states(&mut self) -> Vec<(i32, i32, u8)> {
        let coords = self.played_patterns.iter()
            .map(|pattern| {
                (pattern.start, pattern.end, pattern.index as i32)
            })
            .collect();

        self.playable.led_states(coords)
    }

    pub fn toggle_pattern(&mut self, x: Range<u8>, index: u8) {
        let start = self.playable.ticks_offset() + self.playable.ticks_per_led() * x.start as u32;
        let end = self.playable.ticks_offset() + self.playable.ticks_per_led() * (x.end + 1) as u32;

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
   
    pub fn playing_patterns(&self, patterns: &[Pattern], playing_phrase: &PlayingPhrase) -> Vec<PlayingPattern> {
        // Fill up patterns that are larger as 1 iterationn of pattern with multiple playedpatterns
        // of the same kind
        self.played_patterns.iter()
            .flat_map(|played_pattern| {
                let played_pattern_length = played_pattern.end - played_pattern.start;
                let pattern_length = patterns[played_pattern.index].playable.length;
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
                    PlayingPattern { 
                        // Add phrase start to get ticks that we can compare with cycle
                        start: start + playing_phrase.start,
                        end: end + playing_phrase.start,
                        pattern: played_pattern.index,
                        instrument: playing_phrase.instrument,
                    }
                })
            })
            .collect()
    }
}
