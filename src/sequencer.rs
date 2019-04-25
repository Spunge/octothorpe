
use super::TICKS_PER_BEAT;
use super::handlers::{Cycle, Writer};
use super::message::Message;

#[derive(Debug)]
struct Note {
    // Tick in pattern node is played
    tick: u32,
    // Key in MIDI
    key: u32,
    // Length in ticks
    length: u32,
}

impl Note {
    fn default() -> Self {
        Note {
            tick: 0,
            // A4
            key: 69,
            // Quarter beat
            length: (TICKS_PER_BEAT / 4 as f64) as u32,
        }
    }
}

#[derive(Debug)]
pub struct Pattern {
    // Length in beats
    length: u32,
    notes: Vec<Note>,
}

impl Pattern {
    pub fn output_midi(&self, cycle: Cycle, writer: &mut Writer) {
        for note_event in self.notes.iter() {
        }

        println!("{:?}", cycle.start_tick) 
    }
}

pub struct Sequencer {
    pub pattern: Pattern,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            pattern: Pattern {
                length: 1 * TICKS_PER_BEAT as u32,
                notes: vec![Note::default()],
            },
        }
    }
}
