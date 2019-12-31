
use super::events::*;
use super::TimebaseHandler;

pub trait Loopable {
    type Event: LoopableEvent;

    fn length(&self) -> u32;
    fn events(&mut self) -> &mut Vec<Self::Event>;

    fn clear_events(&mut self) {
        self.events().clear();
    }

    fn try_add_starting_event(&mut self, event: Self::Event) {
        let previous = self.events().iter().filter(|other| other.is_on_same_row(&event)).last();

        if let Some(true) = previous.and_then(|event| Some(event.stop().is_none())) {
            return;
        }

        self.events().push(event);
    }

    fn get_last_event_on_row(&mut self, index: u8) -> Self::Event {
        // What pattern event is this stop for?
        let index = self.events().iter_mut().enumerate()
            .filter(|(_, event)| event.is_on_row(index)).last().unwrap().0;
        
        // Get event from events so we can compare others
        self.events().swap_remove(index)
    }

    fn add_complete_event(&mut self, event: Self::Event) {
        let length = self.length();

        // Remove events that are contained in current event
        self.events().retain(|other| {
            ! event.is_on_same_row(other) || ! event.contains(other, length)
        });

        // Resize events around new event, add new event when previous event is split by current event
        let mut split_events: Vec<Self::Event> = self.events().iter_mut()
            .filter(|other| other.is_on_same_row(&event))
            .filter_map(|other| other.resize_to_fit(&event, length))
            .collect();

        self.events().append(&mut split_events);
        self.events().push(event);
    }

    fn contains_events_starting_between(&mut self, start: u32, stop: u32, index: u8) -> bool {
        self.events().iter()
            .find(|event| event.is_on_row(index) && event.starts_between(start, stop))
            .is_some()
    }

    fn remove_events_starting_between(&mut self, start: u32, stop: u32, index: u8) {
        let indexes: Vec<usize> = self.events().iter().enumerate()
            .filter(|(_, event)| event.is_on_row(index) && event.starts_between(start, stop))
            .map(|(index, _)| index)
            .collect();

        indexes.into_iter().for_each(|index| { self.events().remove(index); () });
    }
}

#[derive(Clone)]
pub struct Phrase {
    // Length in ticks
    length: u32,
    pub pattern_events: Vec<PatternEvent>,
}

impl Loopable for Phrase {
    type Event = PatternEvent;

    fn length(&self) -> u32 { self.length } 
    fn events(&mut self) -> &mut Vec<Self::Event> { &mut self.pattern_events }
}

impl Phrase {
    pub fn new() -> Self {
        Phrase { length: Self::default_length(), pattern_events: vec![] }
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
   
    /*
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
    */
}

#[derive(Clone)]
pub struct Pattern {
    note_events: Vec<NoteEvent>,
    pub is_recording: bool,
}

impl Loopable for Pattern {
    type Event = NoteEvent;

    fn length(&self) -> u32 {
        let max_note = self.note_events.iter()
            .max_by_key(|event| event.start);

        let min = Self::minimum_length();

        if let Some(note) = max_note { 
            (note.start / min + 1) * min
        } else { 
            min
        }
    }

    fn events(&mut self) -> &mut Vec<Self::Event> { &mut self.note_events }
}

impl Pattern {
    fn minimum_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT * 4 }

    pub fn new() -> Self {
        Pattern { note_events: vec![], is_recording: false }
    }

    // Start recording notes from input into pattern
    pub fn switch_recording_state(&mut self) {
        self.is_recording = ! self.is_recording;
    }

    /*
    pub fn quantize(&self, tick: u32, quantize_level: u8) -> u32 {
        let quantize_by_ticks = TimebaseHandler::beats_to_ticks(1.0) / quantize_level as u32;
        let offset = tick % quantize_by_ticks;
    
        if offset < quantize_by_ticks / 2 {
            tick - offset
        } else {
            (tick - offset) + quantize_by_ticks
        }
    }

    pub fn playing_notes(&self, cycle: &Cycle, start: u32, end: u32) -> Vec<(u32, &Note)> {
         self.notes.iter()
            .filter_map(move |note| {
                let note_start = note.start + start;

                // Does note fall in cycle?
                if note_start >= cycle.start && note_start < cycle.end && note_start < end {
                    let delta_ticks = note_start - cycle.start;

                    Some((delta_ticks, note))
                } else {
                    None
                }
            })
            .collect()
    }
    */
}

