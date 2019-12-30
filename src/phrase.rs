
use std::ops::Range;
use std::cmp::Ordering;

use super::events::*;
use super::pattern::{Pattern, PlayedPattern, PlayingPattern};
use super::playable::Playable;
use super::TimebaseHandler;

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

#[derive(Clone)]
pub struct Phrase {
    // Length in ticks
    length: u32,
    pub pattern_events: Vec<PatternEvent>,

    // OOOOOoooldd
    pub playable: Playable,
    pub played_patterns: Vec<PlayedPattern>,
}

impl Phrase {
    fn create(played_patterns: Vec<PlayedPattern>) -> Self {
        Phrase { 
            length: Self::default_length(),
            pattern_events: vec![],

            playable: Playable::new(TimebaseHandler::bars_to_ticks(4), TimebaseHandler::bars_to_ticks(4), 3, 5), 
            played_patterns, 
        }
    }

    pub fn new(index: usize) -> Self {
        Phrase::create(vec![
            PlayedPattern { index, start: TimebaseHandler::bars_to_ticks(0), end: TimebaseHandler::bars_to_ticks(4) },
        ])
    }

    pub fn default_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT * 4 * 4 }
    pub fn set_length(&mut self, length: u32) { 
        self.length = length; 

        // Cut patterns short when shortening length
        self.pattern_events.iter_mut().for_each(|mut event| {
            if let Some(stop) = event.stop {
                if stop > length {
                    event.stop = Some(length);
                }
            }
        });
    }
    pub fn length(&self) -> u32 { self.length } 

    pub fn contains_starting_patterns(&self, start: u32, stop: u32, pattern: usize) -> bool {
        self.pattern_events.iter()
            .find(|event| event.start >= start && event.start < stop && event.pattern == pattern)
            .is_some()
    }

    pub fn remove_patterns_starting_between(&mut self, start: u32, stop: u32, pattern: usize) {
        let indexes: Vec<usize> = self.pattern_events.iter().enumerate()
            .filter(|(_, event)| event.start >= start && event.start < stop && event.pattern == pattern)
            .map(|(index, _)| index)
            .collect();

        indexes.into_iter().for_each(|index| { self.pattern_events.remove(index); () });
    }

    pub fn add_pattern_start(&mut self, start: u32, pattern: usize) {
        let previous = self.pattern_events.iter()
            .filter(|event| event.pattern == pattern).last();

        if let Some(PatternEvent { stop: None, .. }) = previous {
            return;
        }

        self.pattern_events.push(PatternEvent::new(start, None, pattern));
    }

    pub fn add_pattern_stop(&mut self, stop: u32, pattern: usize) {
        // What pattern event is this stop for?
        let index = self.pattern_events.iter_mut().enumerate()
            .filter(|(_, event)| event.pattern == pattern).last().unwrap().0;
        
        let length = self.length();

        // Get event from events so we can compare others
        let mut event = self.pattern_events.swap_remove(index);
        event.stop = Some(stop);

        // Remove events that are contained in current event
        self.pattern_events.retain(|other| {
            event.pattern != other.pattern || ! event.contains(other, length)
        });

        // Resize events around new event, add new event when previous event is split by current event
        let mut split_events: Vec<PatternEvent> = self.pattern_events.iter_mut()
            .filter(|other| event.pattern == other.pattern)
            .filter_map(|other| other.resize_to_fit(&event, length))
            .collect();

        self.pattern_events.append(&mut split_events);
        self.pattern_events.push(event);
        dbg!(&self.pattern_events);
    }

    pub fn clear_pattern_events(&mut self) {
        self.pattern_events = vec![];
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

