
use std::ops::Range;
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

    fn contains_events_starting_in(&mut self, range: Range<u32>, index: u8) -> bool {
        self.events().iter()
            .find(|event| event.is_on_row(index) && range.contains(&event.start()))
            .is_some()
    }

    fn remove_events_starting_in(&mut self, range: Range<u32>, index: u8) {
        let indexes: Vec<usize> = self.events().iter().enumerate()
            .filter(|(_, event)| event.is_on_row(index) && range.contains(&event.start()))
            .map(|(index, _)| index)
            .collect();

        indexes.into_iter().for_each(|index| { self.events().remove(index); () });
    }
}

#[derive(Clone)]
pub struct Phrase {
    // Length in ticks
    length: u32,
    pub pattern_events: Vec<LoopablePatternEvent>,
}

impl Loopable for Phrase {
    type Event = LoopablePatternEvent;

    fn length(&self) -> u32 { self.length } 
    fn events(&mut self) -> &mut Vec<Self::Event> { &mut self.pattern_events }
}

impl Phrase {
    pub fn new() -> Self {
        Phrase { length: Self::default_length(), pattern_events: vec![] }
    }

    pub fn default_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 4 * 4 }
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

    // TODO - This is the simple way of doing things, by not keeping track of playing
    // patterns, which means we can't play parts of patterns over phrase boundaries
    // TODO - There is another way, which involves keeping track of playing events for
    // notes & patterns which keep a reference to loopable event, and adding /removing 
    // these from loopable container together with adding / removing loopable events 
    // based on the references. This will make it possible to play patterns over phrase
    // bounds, i'm not sure if that's the behaviour we want though
    pub fn get_starting_notes(&self, range: Range<u32>, patterns: &[Pattern]) -> Vec<PlayingNoteEvent> {
        self.pattern_events.iter()
            // First check for simple overlap
            // TODO Check if pattern_event is within phrases length ( we can draw after phrase length)
            .filter(|pattern_event| pattern_event.overlaps_tick_range(range.start, range.end))
            .flat_map(|pattern_event| {
                // Looping patterns consist of 2 ranges
                let pattern_event_length = pattern_event.length(self.length());
                // Convert from phrase ticks to note event ticks
                let start_tick = if pattern_event.is_looping() && range.start < pattern_event.start() {
                    pattern_event_length - pattern_event.stop().unwrap()
                } else {
                    0
                };

                patterns[pattern_event.pattern as usize].note_events.iter()
                    .filter(|note_event| note_event.stop().is_some())
                    .filter(move |note_event| {
                        // Is pattern event just starting?
                        let mut note_start_tick = if pattern_event.start() > range.start { 0 } else { range.start - pattern_event.start() };
                        let mut note_stop_tick = range.end - pattern_event.start();
                        // Offset by calculated start tick to grab correct notes from looping patterns
                        note_start_tick += start_tick;
                        note_stop_tick += start_tick;
                        // TODO - Looping patterns with length set explicitly
                        //dbg!(note_start_tick, note_stop_tick, start_tick);
                        (note_start_tick .. note_stop_tick).contains(&note_event.start())
                    })
                    .map(move |note_event| {
                        let mut stop = note_event.stop().unwrap();
                        if note_event.is_looping() { stop += pattern_event_length }

                        PlayingNoteEvent {
                            // subtract start_tick here to make up for the shift in start due
                            // to looping pattern
                            start: pattern_event.start() + note_event.start() - start_tick,
                            stop: pattern_event.start() + stop - start_tick,
                            note: note_event.note,
                            start_velocity: note_event.start_velocity,
                            stop_velocity: note_event.stop_velocity.unwrap(),
                        }
                    })
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct Pattern {
    pub note_events: Vec<LoopableNoteEvent>,
}

impl Loopable for Pattern {
    type Event = LoopableNoteEvent;

    // Pattern will adjust it's length based on the maximum tick it contains
    fn length(&self) -> u32 {
        // Get max tick, stop || start
        let max_tick = self.note_events.iter().map(|event| event.start).max().and_then(|max_start| {
             self.note_events.iter().filter(|event| event.stop.is_some()).map(|event| event.stop.unwrap()).max()
                .and_then(|max_stop| Some(if max_stop > max_start { max_stop } else { max_start }))
                .or_else(|| Some(max_start))
        });

        let mut length = Self::minimum_length();

        if let Some(tick) = max_tick { 
            while length / 2 < tick {
                length = length * 2;
            }
        }

        length
    }

    fn events(&mut self) -> &mut Vec<Self::Event> { &mut self.note_events }
}

impl Pattern {
    fn minimum_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 4 }

    pub fn new() -> Self {
        Pattern { note_events: vec![] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(start: u32, stop: Option<u32>) -> LoopablePatternEvent {
        LoopablePatternEvent { start, stop, pattern: 0 }
    }

    #[test]
    fn length() {
        let mut pattern = Pattern::new();

        let length = Pattern::minimum_length();
        let half_length = length / 2;

        let mut event = LoopableNoteEvent::new(half_length, 1, 1);
        event.set_stop(half_length + 10);

        pattern.add_complete_event(event);
        assert_eq!(pattern.length(), length * 2);

        let mut event = LoopableNoteEvent::new(length, 1, 1);
        event.set_stop(length + 10);

        pattern.add_complete_event(event);
        assert_eq!(pattern.length(), length * 4);
    }
}
