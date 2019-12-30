
use std::ops::Range;
use std::cmp::Ordering;

use super::pattern::{Pattern, PlayedPattern, PlayingPattern};
use super::playable::Playable;
use super::TimebaseHandler;

#[derive(Debug, Clone, Copy)]
pub struct PatternEvent {
    pub start: u32,
    pub stop: Option<u32>,
    pub pattern: usize,
}

impl PatternEvent {
    fn new(start: u32, stop: Option<u32>, pattern: usize) -> Self {
        PatternEvent { start, stop, pattern, }
    }

    pub fn is_looping(&self) -> bool {
        match self.stop {
            Some(stop) => stop <= self.start,
            _ => false
        }
    }

    // Does this event contain another event wholly?
    fn contains(&self, other: &PatternEvent, max_length: u32) -> bool {
        match (self.stop, other.stop) {
            (Some(self_stop), Some(other_stop)) => {
                if ! self.is_looping() && ! other.is_looping() || self.is_looping() && other.is_looping() {
                    // Normal ranges both
                    self.start <= other.start && self_stop >= other_stop
                } else {
                    if self.is_looping() && ! other.is_looping() {
                        self.start <= other.start || self_stop >= other_stop
                    } else {
                        // Normal range can only truly contain a looping range when it's as long as
                        // the container
                        self.start == 0 && self_stop == max_length
                    }
                }
            },
            _ => false,
        }
    }

    // Move out of the way of other event
    fn resize_to_fit(&mut self, other: &PatternEvent, max_length: u32) -> Option<PatternEvent> {
        match (self.stop, other.stop) {
            (Some(self_stop), Some(other_stop)) => {
                let starts_before = self.start < other.start;
                let stops_before = self_stop <= other_stop;

                let stops_after = self_stop > other_stop;
                let starts_after = self.start >= other.start;

                //       [    other   ]
                // [    self    ]              
                let end_overlaps = self_stop > other.start && (stops_before || starts_before);
                // [    other   ]
                //       [    self    ]
                let begin_overlaps = self.start < other_stop && (stops_after || starts_after);
    
                match (begin_overlaps, end_overlaps) {
                    // Only begin overlaps
                    (true, false) => { self.start = other_stop; None },
                    // Only end overlaps
                    (false, true) => { self.stop = Some(other.start); None },
                    // They both overlap || don't overlap
                    // Could be valid note placement || split, depending on looping or not
                    _ => {
                        if self.contains(other, max_length) {
                            // Create split pattern event
                            let event = PatternEvent::new(other_stop, self.stop, self.pattern);
                            // Adjust own event
                            self.stop = Some(other.start);
                            Some(event)
                        } else {
                            None
                        }
                    },
                }
            },
            _ => None,
        }
    }
}

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
    pub fn set_length(&mut self, length: u32) { self.length = length; }
    pub fn length(&self) -> u32 { self.length } 

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
        
        // Get event from events so we can compare others
        let mut event = self.pattern_events.swap_remove(index);
        event.stop = Some(stop);

        let length = self.length();

        // Remove events that are contained in current event
        self.pattern_events.retain(|other| {
            event.pattern != other.pattern || ! event.contains(other, length)
        });

        // Resize events around new event, add new event when previous event is split by current event
        let mut split_events: Vec<PatternEvent> = self.pattern_events.iter_mut()
            .filter(|other| event.pattern == other.pattern)
            // Is event split by current event?
            // Create 2 events for events that are split by current event
            .filter_map(|other| other.resize_to_fit(&event, length))
            .collect();

        if split_events.len() > 0 {
            dbg!(&event);
            dbg!(&split_events);
        }

        self.pattern_events.append(&mut split_events);
        self.pattern_events.push(event);

        dbg!(&self.pattern_events);
    }

    // TODO - Smart cut of |  stop]    [start       |
    /*
    pub fn add_pattern(&mut self, start_tick: u32, stop_tick: u32, pattern: usize) { 
        // Compare overlap of previous event pairs with current events
        // Chunks will always be in start => stop events, ticks of start can be after stop though
        while let Some([start, stop]) = chunks.next() {
            // Yeah i know this is double check..
            let is_same_pattern = start.pattern == pattern && stop.pattern == pattern;
            let overlaps = start.tick >= start_tick && stop.tick <= stop_tick;
            let overlaps_end = start.tick < start_tick && stop.tick > start_tick;
            let overlaps_start = start.tick < stop_tick && stop.tick > stop_tick;

            if is_same_pattern {
                if overlaps {
                    // Previous event is completely within current event
                    continue;
                } else if overlaps_end {
                    // Overlap of end of previous note
                    stop.tick = start_tick;
                } else if overlaps_start {
                    // Overlap of start of previous note
                    start.tick = stop_tick;
                }
            }

            retained.push(*start);
            retained.push(*stop);
        }

        self.pattern_events = retained;
        self.pattern_events.push(PatternEvent::new)
        dbg!(&self.pattern_events);
    }
    */

    pub fn clear_pattern_events(&mut self) {
        self.pattern_events = vec![];
    }

    // TODO - when shortening length, notes that are longer as playable length
    // should be cut shorter aswell
    pub fn change_length(&mut self, length_modifier: u32) {
        let current_modifier = self.playable.length_modifier();
        let current_length = self.playable.length;

        if let Some(next_modifier) = self.playable.change_length(length_modifier) {
            // Add to current patterns
            if current_modifier < next_modifier {
                let times = next_modifier / current_modifier;

                let played_patterns: Vec<PlayedPattern> = (1..times).into_iter()
                    .flat_map(|multiplier| -> Vec<PlayedPattern> {
                        self.played_patterns.iter()
                            .map(|played_pattern| played_pattern.clone())
                            .map(|mut played_pattern| { 
                                played_pattern.start = played_pattern.start + multiplier * current_length;
                                played_pattern.end = played_pattern.end + multiplier * current_length;
                                played_pattern
                            })
                            .collect()
                    })
                    .collect();

                self.played_patterns.extend(played_patterns);
            } 

            // Cut from current patterns
            if current_modifier > next_modifier {
                let new_length = next_modifier * self.playable.minimum_length;

                self.played_patterns.retain(|played_pattern| {
                    played_pattern.start < new_length
                });

                self.played_patterns.iter_mut().for_each(|played_pattern| {
                    if played_pattern.end > new_length {
                        played_pattern.end = new_length;
                    }
                });
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains() {
        let no_end = PatternEvent::new(0, None, 0);
        let normal = PatternEvent::new(50, Some(150), 0);
        let looping = PatternEvent::new(150, Some(50), 0);

        assert_eq!(no_end.contains(&normal, 200), false);
        assert_eq!(no_end.contains(&looping, 200), false);
        assert_eq!(normal.contains(&looping, 200), false);
        assert_eq!(normal.contains(&no_end, 200), false);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(100), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(100, Some(150), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(150), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(150), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(50, Some(150), 0), 200), false);
        assert_eq!(looping.contains(&PatternEvent::new(150, Some(170), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(20, Some(50), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(160, Some(40), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(150, Some(50), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(120, Some(50), 0), 200), false);
        assert_eq!(looping.contains(&PatternEvent::new(150, None, 0), 200), false);
    }

    #[test]
    fn resize_to_fit() {
        let mut no_end = PatternEvent::new(0, None, 0);
        let mut looping = PatternEvent::new(150, Some(50), 0);

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(100, Some(150), 0), 200);
        assert_eq!((50, Some(100)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(0, Some(30), 0), 200);
        assert_eq!((50, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(50, Some(100), 0), 200);
        assert_eq!((100, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(100, Some(170), 0), 200);
        assert_eq!((170, Some(50)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(40, Some(100), 0), 200);
        assert_eq!((150, Some(40)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        //let mut event = PatternEvent::new(150, Some(50), 0);
        //let split = event.resize_to_fit(&PatternEvent::new(60, Some(100), 0), 200);
        //assert_eq!((150, Some(50)), (event.start, event.stop));
        //assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(80, Some(100), 0), 200);
        assert_eq!((50, Some(80)), (event.start, event.stop));
        assert_eq!(Some((100, Some(150))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(20, Some(40), 0), 200);
        assert_eq!((150, Some(20)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(170, Some(40), 0), 200);
        assert_eq!((150, Some(170)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(160, Some(180), 0), 200);
        assert_eq!((150, Some(160)), (event.start, event.stop));
        assert_eq!(Some((180, Some(50))), split.and_then(|e| Some((e.start, e.stop))));
    }
}

