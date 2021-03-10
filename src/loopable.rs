
use crate::*;

pub trait Loopable {
    type Event: LoopableEvent;

    fn length(&self) -> u32;
    fn events(&self) -> &Vec<Self::Event>;
    fn events_mut(&mut self) -> &mut Vec<Self::Event>;

    fn clear_events(&mut self) {
        self.events_mut().clear();
    }

    fn try_add_starting_event(&mut self, event: Self::Event) {
        let previous = self.events_mut().iter().filter(|other| other.is_on_same_row(&event)).last();

        if let Some(true) = previous.and_then(|event| Some(event.stop().is_none())) {
            return;
        }

        self.events_mut().push(event);
    }

    fn get_last_event_on_row(&mut self, index: u8) -> Self::Event {
        // What pattern event is this stop for?
        let index = self.events_mut().iter_mut().enumerate()
            .filter(|(_, event)| event.is_on_row(index)).last().unwrap().0;
        
        // Get event from events so we can compare others
        self.events_mut().swap_remove(index)
    }

    fn add_complete_event(&mut self, event: Self::Event) {
        let length = self.length();

        // Remove events that are contained in current event
        self.events_mut().retain(|other| {
            ! event.is_on_same_row(other) || ! event.contains(other, length)
        });

        // Resize events around new event, add new event when previous event is split by current event
        let mut split_events: Vec<Self::Event> = self.events_mut().iter_mut()
            .filter(|other| other.is_on_same_row(&event))
            .filter_map(|other| other.resize_to_fit(&event, length))
            .collect();

        self.events_mut().append(&mut split_events);
        self.events_mut().push(event);
    }

    fn contains_events_starting_in(&mut self, range: TickRange, index: u8) -> bool {
        self.events_mut().iter()
            .find(|event| event.is_on_row(index) && range.contains(event.start()))
            .is_some()
    }

    fn remove_events_starting_in(&mut self, range: TickRange, index: u8) {
        let indexes: Vec<usize> = self.events_mut().iter().enumerate()
            .filter(|(_, event)| event.is_on_row(index) && range.contains(event.start()))
            .map(|(index, _)| index)
            .collect();

        indexes.into_iter().for_each(|index| { self.events_mut().remove(index); () });
    }

    /*
     * We want to loop phrases/patterns that are shorter as container phrase / pattern_event
     */
    fn looping_ranges(&self, range: &TickRange) -> Vec<(TickRange, u32)> {
        let iteration = range.start / self.length();
        let start = range.start % self.length();

        // Sequence range will stop exactly at phrase length
        let mut stop = range.stop % self.length();
        if stop == 0 {
            stop = self.length();
        }

        if start > stop {
            vec![
                (TickRange::new(start, self.length()), iteration * self.length()), 
                (TickRange::new(0, stop), (iteration + 1) * self.length())
            ]
        } else {
            vec![(TickRange::new(start, stop), iteration * self.length())]
        }
    }
}

#[derive(Clone)]
pub struct Timeline {
    pub phrase_events: Vec<LoopablePhraseEvent>,
}

impl Loopable for Timeline {
    type Event = LoopablePhraseEvent;

    // Get max stop tick & add some padding
    fn length(&self) -> u32 { 
        let max_stop_tick = self.phrase_events.iter()
            .filter(|phrase_event| phrase_event.stop.is_some())
            .map(|phrase_event| phrase_event.stop.unwrap())
            .max();

        if max_stop_tick.is_some() {
            max_stop_tick.unwrap() + Phrase::default_length() * 4
        } else {
            Phrase::default_length() * 8
        }
    } 

    fn events(&self) -> &Vec<Self::Event> { &self.phrase_events }
    fn events_mut(&mut self) -> &mut Vec<Self::Event> { &mut self.phrase_events }
}

impl Timeline {
    pub fn new() -> Self {
        Timeline { phrase_events: vec![] }
    }

    pub fn get_last_stop(&self) -> u32 {
        self.events().iter().filter(|event| event.stop.is_some()).map(|event| event.stop.unwrap()).max()
            .or_else(|| Some(0))
            .unwrap()
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
    fn events(&self) -> &Vec<Self::Event> { &self.pattern_events }
    fn events_mut(&mut self) -> &mut Vec<Self::Event> { &mut self.pattern_events }
}

impl Phrase {
    pub fn new() -> Self {
        Phrase { length: Self::default_length(), pattern_events: vec![] }
    }

    // Default phrase length = 4 bars
    pub fn default_length() -> u32 { Transport::TICKS_PER_BEAT as u32 * 4 * 4 }
    pub fn set_length(&mut self, length: u32) { 
        self.length = length; 

        // Remove pattern events that start outside of length
        self.pattern_events.retain(|event| {
            event.start() < length
        });

        // Cut pattern events short that start within length but stop after 
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


            2 * Self::minimum_length() + max_tick.or(Some(0)).unwrap()
        })
    }

    fn events(&self) -> &Vec<Self::Event> { &self.note_events }
    fn events_mut(&mut self) -> &mut Vec<Self::Event> { &mut self.note_events }
}

impl Pattern {
    pub fn minimum_length() -> u32 { Transport::TICKS_PER_BEAT as u32 * 4 }

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

    pub fn starting_notes(&self, absolute_start: u32, relative_range: TickRange, pattern_event_length: u32) 
        -> Vec<PlayingNoteEvent> 
    {
        // Get looping ranges when pattern is a looping pattern
        let ranges = if ! self.has_explicit_length() { vec![(relative_range, 0)] } else { self.looping_ranges(&relative_range) };

        ranges.iter()
            .flat_map(|(range, offset)| {
                self.note_events.iter()
                    .filter(move |note_event| {
                        range.contains(note_event.start())
                    })
                    .map(move |note_event| {
                        let looping_note_length = if self.has_explicit_length() { self.length() } else { pattern_event_length };
                        let note_start = offset + note_event.start();
                        let note_stop = offset + note_event.stop().unwrap() + if note_event.is_looping() { looping_note_length } else { 0 };
                        let start_tick = note_start - relative_range.start;
                        let stop_tick = note_stop - relative_range.start;

                        PlayingNoteEvent {
                            start: absolute_start + start_tick,
                            stop: absolute_start + stop_tick,
                            note: note_event.note,
                            start_velocity: note_event.start_velocity,
                            stop_velocity: note_event.stop_velocity.unwrap(),
                        }
                    })
            })
            .collect()
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
