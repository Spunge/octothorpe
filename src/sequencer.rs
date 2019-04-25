
use super::TICKS_PER_BEAT;
use super::handlers::{Cycle, Writer};
use super::message::Message;

#[derive(Debug, Clone, Copy)]
struct Note {
    // Ticks in pattern that note should be played
    pub note_on: f64,
    pub length: f64,

    key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    fn new(note_on: f64, length: f64) -> Self {
        Note { note_on, length, key: 69, velocity: 127 }
    }

    fn note_on(&self) -> Message {
        Message::Note( 
            0, 
            [0x90, self.key, self.velocity],
        )
    }
    
    fn note_off(&self) -> Message {
        Message::Note( 
            0, 
            [0x80, self.key, self.velocity],
        )
    }
}

#[derive(Debug)]
struct PlayedNote {
    note: Note,
    note_off: f64,
}

impl PlayedNote {
    fn new(note: Note, ticks_elapsed: &f64) -> Self {
        PlayedNote { note, note_off: ticks_elapsed + note.length }
    }
}

#[derive(Debug)]
pub struct Pattern {
    // Length in beats
    length: f64,
    notes: Vec<Note>,
    played_notes: Vec<PlayedNote>,
}

impl Pattern {
    pub fn output_note_on(&mut self, cycle: &Cycle, ticks_elapsed: &f64, writer: &mut Writer) {
        for note in self.notes.iter() {
            let start_tick = cycle.start_tick % self.length;
            let end_tick = start_tick + cycle.ticks_in_cycle;

            let a = note.note_on >= start_tick && note.note_on < end_tick;
            // Could be note starts at tick 0 and end tick is after 
            let b = end_tick > self.length && note.note_on < end_tick % self.length;

            // Does note start fall in this cycle?
            if a || b {
                // TODO - Get note ticks offset && note frame offset
                // TODO - Remove notes with same key from playednotes as MIDI will stop the played
                // note
                writer.write(note.note_on());
                let played_note = PlayedNote::new(*note, ticks_elapsed);
                self.played_notes.push(played_note);
            }
        }

    }

    pub fn output_note_off(&mut self, cycle: &Cycle, ticks_elapsed: &f64, writer: &mut Writer) {
        self.played_notes.retain(|played_note| {
            let a = played_note.note_off >= *ticks_elapsed 
                && played_note.note_off < ticks_elapsed + cycle.ticks_in_cycle;

            if a {
                writer.write(played_note.note.note_off());
            }

            // Return the opposite of A to keep notes that are not yet finished
            !a
        });
    }
}

pub struct Sequencer {
    pub pattern: Pattern,
    // Keep track of elapsed ticks to trigger note_off when transport stops
    pub ticks_elapsed: f64,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            ticks_elapsed: 0.0,

            pattern: Pattern {
                length: 1.0 * TICKS_PER_BEAT,
                played_notes: Vec::new(),
                notes: vec![
                    Note::new(0.0, TICKS_PER_BEAT),
                    //Note::new(TICKS_PER_BEAT / 2.0, TICKS_PER_BEAT / 1.0)
                ],
            },
        }
    }

    pub fn update(&mut self, cycle: &Cycle) {
        self.ticks_elapsed += cycle.ticks_in_cycle;
    }

    // This is only called when transport is running
    pub fn output_midi_note_on(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.pattern.output_note_on(cycle, &self.ticks_elapsed, writer);
    }

    // This is always called, also when transport is not running
    pub fn output_midi_note_off(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.pattern.output_note_off(cycle, &self.ticks_elapsed, writer);
        self.update(cycle);
    }
}
