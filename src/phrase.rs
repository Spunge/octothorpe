
use std::ops::Range;
use std::cmp::Ordering;

use super::pattern::{Pattern, PlayedPattern, PlayingPattern};
use super::playable::Playable;
use super::TimebaseHandler;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PatternEventType {
    Start,
    Stop,
}
impl Ord for PatternEventType {
    fn cmp(&self, other: &PatternEventType) -> Ordering {
        if let (PatternEventType::Stop, PatternEventType::Start) = (self, other) {
            Ordering::Less
        } else if let (PatternEventType::Start, PatternEventType::Stop) = (self, other) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}
impl PartialOrd for PatternEventType {
    fn partial_cmp(&self, other: &PatternEventType) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PatternEvent {
    pub event_type: PatternEventType,
    pub tick: u32,
    pub pattern: usize,
}

// Order pattern events by ticks
impl Ord for PatternEvent {
    fn cmp(&self, other: &PatternEvent) -> Ordering {
        self.tick.cmp(&other.tick).then(self.event_type.cmp(&other.event_type))
    }
}
impl PartialOrd for PatternEvent {
    fn partial_cmp(&self, other: &PatternEvent) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
//impl PartialEq for PatternEvent {
    //fn eq(&self, other: &PatternEvent) -> bool {
        //self.tick == other.tick && self.event_type == other.event_type && 
    //}
//}

impl PatternEvent {
    pub fn start(tick: u32, pattern: usize) -> Self {
        Self { tick, pattern, event_type: PatternEventType::Start }
    }

    pub fn stop(tick: u32, pattern: usize) -> Self {
        Self { tick, pattern, event_type: PatternEventType::Stop }
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

    // TODO - Smart cut of |  stop]    [start       |
    pub fn add_pattern(&mut self, start_tick: u32, stop_tick: u32, pattern: usize) { 
        // Compare overlap of previous event pairs with current events
        // Chunks will always be in start => stop events, ticks of start can be after stop though
        let mut chunks = self.pattern_events.chunks_mut(2);
        let mut retained: Vec<PatternEvent> = vec![];

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
        self.pattern_events.push(PatternEvent::start(start_tick, pattern));
        self.pattern_events.push(PatternEvent::stop(stop_tick, pattern));
        dbg!(&self.pattern_events);
    }

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
