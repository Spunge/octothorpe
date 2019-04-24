
use super::TICKS_PER_BEAT;

#[derive(Debug)]
struct Note {
    // Key in MIDI
    key: u32,
    // Length in ticks
    length: u32,
}

impl Note {
    fn default() -> Self {
        Note {
            // A4
            key: 69,
            // Quarter beat
            length: (TICKS_PER_BEAT / 4 as f64) as u32,
        }
    }
}

#[derive(Debug)]
struct NoteEvent {
    tick: u32,
    note: Note,
}

#[derive(Debug)]
struct Pattern {
    // Length in beats
    length: u32,
    notes: Vec<NoteEvent>,
}

#[derive(Debug)]
pub struct Sequencer {
    pattern: Pattern,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            pattern: Pattern {
                length: 1,
                notes: vec![NoteEvent{ tick: 0, note: Note::default() }]
            }
        }
    }
}
