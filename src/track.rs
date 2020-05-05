
use super::TickRange;
use super::port::*;
use super::loopable::*;
use super::cycle::*;
use super::events::*;
use super::message::*;

pub struct Track {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],

    playing_notes: Vec<PlayingNoteEvent>,

    //pub knob_group: u8,
    //knob_values: [u8; 128],

    output: MidiOut,
}

impl Track {
    pub fn new(client: &jack::Client, id: u8) -> Self {
        let patterns = [Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new()];
        let phrases = [Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new()];

        //let input = client.register_port("APC20 in", jack::MidiIn::default()).unwrap();
        let output = client.register_port(format!("Track {}", id).as_str(), jack::MidiOut::default()).unwrap();

        Track {
            phrases,
            patterns,

            playing_notes: vec![],

            // There's 4 knob groups, this way we can have knobs * 4 !
            //knob_group: 0,
            //knob_values: [0; 128],

            output: MidiOut::new(output),
        }
    }

    pub fn pattern(&self, index: u8) -> &Pattern { &self.patterns[index as usize] }
    pub fn pattern_mut(&mut self, index: u8) -> &mut Pattern { &mut self.patterns[index as usize] }

    pub fn phrase(&self, index: u8) -> &Phrase { &self.phrases[index as usize] }
    pub fn phrase_mut(&mut self, index: u8) -> &mut Phrase { &mut self.phrases[index as usize] }

    pub fn clone_pattern(&mut self, from: u8, to: u8) {
        self.patterns[to as usize] = self.patterns[from as usize].clone();
    }

    pub fn clone_phrase(&mut self, from: u8, to: u8) {
        self.phrases[to as usize] = self.phrases[from as usize].clone();
    }

    pub fn clear_playing_notes(&mut self) {
        self.playing_notes = vec![];
    }

    // Start all notes in playing notes array. Used when starting mid-track
    pub fn start_playing_notes(&mut self, cycle: &ProcessCycle) {
        let mut messages = self.playing_notes.iter()
            .map(|note| TimedMessage::new(0, Message::Note([0x90, note.note, note.start_velocity])))
            .collect();

        self.output.write_midi(cycle.scope, &mut messages);
    }

    // Stop playing notes, used when stopping mid-track
    pub fn stop_playing_notes(&mut self, cycle: &ProcessCycle) {
        let mut messages = self.playing_notes.iter()
            .map(|note| TimedMessage::new(0, Message::Note([0x80, note.note, note.stop_velocity])))
            .collect();

        self.output.write_midi(cycle.scope, &mut messages);
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle, starting_notes: Vec<PlayingNoteEvent>) {
        // Always play note off messages
        let mut messages = vec![];

        self.playing_notes.retain(|note| {
            // Play & remove notes that fall in cycle
            if cycle.tick_range.contains(note.stop) {
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

        //println!("{:?}", messages);
        //println!("{:?}", self.playing_notes);

        // Remember playing notes to later trigger note off message & output note on messages
        self.playing_notes.extend(starting_notes);

        // Output note off mesassages && write midi
        //self.output.output_messages(&mut messages);
        self.output.write_midi(cycle.scope, &mut messages);
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