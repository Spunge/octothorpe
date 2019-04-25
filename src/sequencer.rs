
use super::TICKS_PER_BEAT;
use super::handlers::{Cycle, Writer};
use super::message::Message;

#[derive(Debug)]
struct Note {
    // Ticks in pattern that note should be played
    note_on: f64,
    note_off: f64,

    key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    fn new(note_on: f64, length: f64) -> Self {
        Note {
            note_on,
            note_off: note_on + length,
            key: 69,
            velocity: 127,
        }
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
pub struct Pattern {
    // Length in beats
    length: f64,
    notes: Vec<Note>,
}

impl Pattern {
    pub fn output_midi(&self, cycle: Cycle, writer: &mut Writer) {
        let ticks_in_cycle = cycle.end_tick - cycle.start_tick;

        let start_tick = cycle.start_tick % self.length;
        let end_tick = start_tick + ticks_in_cycle;

        for note in self.notes.iter() {
            let a = note.note_on >= start_tick && note.note_on < end_tick;
            // Could be note starts at tick 0 and end tick is after 
            let b = end_tick > self.length && note.note_on < end_tick % self.length;

            // Does note start fall in this cycle?
            if a || b {
                writer.write(note.note_on());
            }

            if note.note_off >= start_tick && note.note_off < end_tick {
                writer.write(note.note_off());
            }
        }
    }
}

pub struct Sequencer {
    pub pattern: Pattern,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            pattern: Pattern {
                length: 1.0 * TICKS_PER_BEAT,
                notes: vec![
                    Note::new(0.0, TICKS_PER_BEAT / 4.0),
                    Note::new(TICKS_PER_BEAT / 2.0, TICKS_PER_BEAT / 4.0)
                ],
            },
        }
    }
}
