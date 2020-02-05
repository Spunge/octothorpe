
use std::ops::Range;
use super::port::*;
use super::loopable::*;
use super::cycle::*;
use super::events::*;
use super::message::*;

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],

    playing_notes: Vec<PlayingNoteEvent>,

    //pub knob_group: u8,
    //knob_values: [u8; 128],

    output: MidiOut,
}

impl Instrument {
    pub fn new(client: &jack::Client, id: u8) -> Self {
        let patterns = [Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new()];
        let phrases = [Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new()];

        //let input = client.register_port("APC20 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port(format!("Instrument {}", id).as_str(), jack::MidiOut::default()).unwrap();

        Instrument {
            phrases,
            patterns,

            playing_notes: vec![],

            // There's 4 knob groups, this way we can have knobs * 4 !
            //knob_group: 0,
            //knob_values: [0; 128],

            output: MidiOut::new(output),
        }
    }

    pub fn get_pattern(&mut self, index: u8) -> &mut Pattern {
        &mut self.patterns[index as usize]
    }

    pub fn phrase(&self, index: u8) -> &Phrase { &self.phrases[index as usize] }
    pub fn phrase_mut(&mut self, index: u8) -> &mut Phrase { &mut self.phrases[index as usize] }

    pub fn clone_pattern(&mut self, from: u8, to: u8) {
        self.patterns[to as usize] = self.patterns[from as usize].clone();
    }

    pub fn clone_phrase(&mut self, from: u8, to: u8) {
        self.phrases[to as usize] = self.phrases[from as usize].clone();
    }

    pub fn starting_notes(&self, range: Range<u32>, sequence_start: u32, phrase_index: u8) -> Vec<PlayingNoteEvent> {
        let phrase = self.phrase(phrase_index);

        let phrase_start_tick = (range.start - sequence_start) % phrase.length();
        let iteration = (range.start - sequence_start) / phrase.length();
        let mut phrase_stop_tick = (range.end - sequence_start) % phrase.length();
        if phrase_stop_tick == 0 { 
            phrase_stop_tick = phrase.length();
        }

        phrase.pattern_events.iter()
            .filter(|pattern_event| pattern_event.overlaps_tick_range(phrase_start_tick, phrase_stop_tick))
            .flat_map(|pattern_event| {
                // Looping patterns consist of 2 ranges
                let pattern_event_length = pattern_event.length(phrase.length());
                // Convert from phrase ticks to note event ticks
                let start_tick = if pattern_event.is_looping() && phrase_start_tick < pattern_event.start() {
                    pattern_event_length - pattern_event.stop().unwrap()
                } else {
                    0
                };

                self.patterns[pattern_event.pattern as usize].note_events.iter()
                    .filter(|note_event| note_event.stop().is_some())
                    .filter(move |note_event| {
                        // Is pattern event just starting?
                        let mut note_start_tick = if pattern_event.start() > phrase_start_tick { 0 } else { phrase_start_tick - pattern_event.start() };
                        let mut note_stop_tick = phrase_stop_tick - pattern_event.start();
                        // Offset by calculated start tick to grab correct notes from looping patterns
                        note_start_tick += start_tick;
                        note_stop_tick += start_tick;
                        // TODO - Looping patterns with length set explicitly
                        //dbg!(note_start_tick, note_stop_tick, start_tick);
                        (note_start_tick .. note_stop_tick).contains(&note_event.start())
                    })
                    .map(move |note_event| {
                        let base_tick = sequence_start + iteration * phrase.length() + pattern_event.start();
                        let mut stop = note_event.stop().unwrap();
                        if note_event.is_looping() { stop += pattern_event_length }

                        PlayingNoteEvent {
                            // subtract start_tick here to make up for the shift in start due
                            // to looping pattern
                            start: base_tick + note_event.start() - start_tick,
                            stop: base_tick + stop - start_tick,
                            note: note_event.note,
                            start_velocity: note_event.start_velocity,
                            stop_velocity: note_event.stop_velocity.unwrap(),
                        }
                    })
            })
            .collect()
    }

    /*
    fn starting_notes(&self, range: &Range<u32>, sequence_start: u32, phrase: u8) -> Vec<PlayingNoteEvent> {
        let phrase = &self.phrases[phrase as usize];
        let iteration = (range.start - sequence_start) / phrase.length();
        // TODO Not every iteration comes down to 0 as phrase_tick_start
        // handle this in seuqencer, asking notes from correct phrases
        let phrase_start_tick = (range.start - sequence_start) % phrase.length();
        let phrase_stop_tick = (range.end - sequence_start) % phrase.length();

        // TODO - This is the simple way of doing things, by not keeping track of playing
        // patterns, which means we can't play parts of patterns over phrase boundaries
        // TODO - There is another way, which involves keeping track of playing events for
        // notes & patterns which keep a reference to loopable event, and adding /removing 
        // these from loopable container together with adding / removing loopable events 
        // based on the references. This will make it possible to play patterns over phrase
        // bounds, i'm not sure if that's the behaviour we want though
        phrase.pattern_events.iter()
            // First check for simple overlap
            // TODO Check if pattern_event is within phrases length ( we can draw after phrase length)
            .filter(|pattern_event| pattern_event.overlaps_tick_range(phrase_start_tick, phrase_stop_tick))
            .flat_map(|pattern_event| {
                // Looping patterns consist of 2 ranges
                let pattern_event_length = pattern_event.length(phrase.length());
                // Convert from phrase ticks to note event ticks
                let start_tick = if pattern_event.is_looping() && phrase_start_tick < pattern_event.start() {
                    pattern_event_length - pattern_event.stop().unwrap()
                } else {
                    0
                };

                self.patterns[pattern_event.pattern as usize].note_events.iter()
                    .filter(|note_event| note_event.stop().is_some())
                    .filter(move |note_event| {
                        // Is pattern event just starting?
                        let mut note_start_tick = if pattern_event.start() > phrase_start_tick { 0 } else { phrase_start_tick - pattern_event.start() };
                        let mut note_stop_tick = phrase_stop_tick - pattern_event.start();
                        // Offset by calculated start tick to grab correct notes from looping patterns
                        note_start_tick += start_tick;
                        note_stop_tick += start_tick;
                        // TODO - Looping patterns with length set explicitly
                        //dbg!(note_start_tick, note_stop_tick, start_tick);
                        (note_start_tick .. note_stop_tick).contains(&note_event.start())
                    })
                    .map(move |note_event| {
                        let base_tick = sequence_start + iteration * phrase.length() + pattern_event.start();
                        let mut stop = note_event.stop().unwrap();
                        if note_event.is_looping() { stop += pattern_event_length }

                        PlayingNoteEvent {
                            // subtract start_tick here to make up for the shift in start due
                            // to looping pattern
                            start: base_tick + note_event.start() - start_tick,
                            stop: base_tick + stop - start_tick,
                            note: note_event.note,
                            start_velocity: note_event.start_velocity,
                            stop_velocity: note_event.stop_velocity.unwrap(),
                        }
                    })
            })
            .collect()
    }
    */

    // TODO - Don't pass cycle directly, handle changing phrases etc in sequencer
    pub fn output_midi(&mut self, cycle: &ProcessCycle, starting_notes: Vec<PlayingNoteEvent>) {
        // Always play note off messages
        let mut messages = vec![];

        self.playing_notes.retain(|note| {
            // Play & remove notes that fall in cycle
            if cycle.tick_range.contains(&note.stop) {
                let frame = cycle.tick_to_frame(note.stop);
                messages.push(TimedMessage::new(frame, Message::Note([0x80, note.note, note.stop_velocity])));
                false
            } else {
                true
            }
        });

        // Create actual midi from note representations
        let note_on = starting_notes.iter()
            .map(|note| {
                let frame = cycle.tick_to_frame(note.start);
                TimedMessage::new(frame, Message::Note([0x90, note.note, note.start_velocity]))
            });

        messages.extend(note_on);

        // Remember playing notes to later trigger note off message & output note on messages
        self.playing_notes.extend(starting_notes);

        // Output note off mesassages && write midi
        self.output.output_messages(&mut messages);
        self.output.write_midi(cycle.scope);
    }

    /*
    pub fn switch_knob_group(&mut self, group: u8) {
        self.knob_group = group;
    }

    pub fn set_knob_value(&mut self, index: u8, value: u8) -> u8 {
        let knob = self.knob_group * 16 + index;
        self.knob_values[knob as usize] = value;
        knob
    }

    pub fn get_knob_values(&self) -> &[u8] {
        let start = self.knob_group as usize * 16;
        let end = start as usize + 16;
        &self.knob_values[start .. end]
    }

    pub fn knob_value_changed(&mut self, knob: u8, value: u8) -> Option<u8> {
        if self.knob_values[knob as usize] != value {
            self.knob_values[knob as usize] = value;
            Some(value)
        } else {
            None
        }
    }
    */
}
