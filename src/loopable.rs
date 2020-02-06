
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

    // Get ranges of this phrase that should be played when keeping looping in mind
    fn relative_ranges(&self, range: &Range<u32>) -> Vec<(Range<u32>, u32)> {
        let iteration = range.start / self.length();
        let offset = iteration * self.length();
        let start = range.start % self.length();

        // Range will stop exactly at phrase length
        let mut stop = range.end % self.length();
        if stop == 0 {
            stop = self.length();
        }

        if start > stop {
            vec![(start .. self.length(), offset), (0 .. stop, offset)]
        } else {
            vec![(start .. stop, offset)]
        }
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

    // Default phrase length = 4 bars
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
}

#[derive(Clone)]
pub struct Pattern {
    pub note_events: Vec<LoopableNoteEvent>,
    pub length: Option<u32>,
}

impl Loopable for Pattern {
    type Event = LoopableNoteEvent;

    // Pattern will adjust it's length based on the maximum tick it contains
    fn length(&self) -> u32 {
        // When length is not set explicitely, calculate it based on notes in pattern so that we
        // have an indication
        self.length.unwrap_or_else(|| {
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
        })
    }

    fn events(&mut self) -> &mut Vec<Self::Event> { &mut self.note_events }

    // Only return relative ranges when this is a looping pattern
    fn relative_ranges(&self, range: &Range<u32>) -> Vec<(Range<u32>, u32)> {
        if self.has_explicit_length() {
            Loopable::relative_ranges(self, range)
        } else {
            vec![(range.clone(), 0)]
        }
    }
}

impl Pattern {
    pub fn minimum_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT as u32 * 4 }

    pub fn new() -> Self {
        Pattern { note_events: vec![], length: None }
    }

    pub fn has_explicit_length(&self) -> bool {
        self.length.is_some()
    }

    pub fn unset_length(&mut self) {
        self.length = None;
    }

    pub fn set_length(&mut self, length: u32) {
        self.length = Some(length);
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
