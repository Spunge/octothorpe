
use super::TICKS_PER_BEAT;
use super::handlers::{Cycle, Writer};
use super::message::{Message, MessageData};

#[derive(Debug, Clone, Copy)]
struct Note {
    // Ticks in pattern that note should be played
    pub tick: f64,
    pub length: f64,

    key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    fn new(tick: f64, length: f64, key: u8) -> Self {
        Note { tick, length, key: key, velocity: 127 }
    }

    fn note_on(&self, time: u32) -> Message {
        Message::new(time, MessageData::Note([0x90, self.key, self.velocity]))
    }
    
    fn note_off(&self, time: u32) -> Message {
        Message::new(time, MessageData::Note([0x80, self.key, self.velocity]))
    }
}

#[derive(Debug)]
struct PlayedNote {
    note: Note,
    note_off: f64,
}

impl PlayedNote {
    fn new(note: Note, note_off: f64) -> Self {
        PlayedNote { note, note_off }
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
    pub fn output_note_on_events_in_cycle(&mut self, cycle: &Cycle, ticks_elapsed: &f64, writer: &mut Writer) {
        let start_tick = cycle.start_tick % self.length;
        let end_tick = start_tick + cycle.ticks_in_cycle;
        
        // Clone so we can change the tick on notes for next pattern iteration
        let mut played_notes = self.notes.iter()
            .cloned()
            // If note in next iteration of the patters does belong in this cycle, add it
            .map(|mut note| {
                if note.tick + self.length >= start_tick 
                    && note.tick + self.length < end_tick 
                {
                    note.tick += self.length;
                }
                note
            })
            // Check all notes to see if they belong in this cycle
            .filter(|note| {
                note.tick >= start_tick && note.tick < end_tick
            })
            // Play notes
            .map(|note| {
                let ticks_till_note = note.tick - start_tick;
                let frames_till_note = ticks_till_note / cycle.ticks_in_cycle * cycle.frames as f64;
                let note_off = ticks_elapsed + note.length + ticks_till_note;

                // Write note
                writer.write(note.note_on(frames_till_note.round() as u32));

                PlayedNote::new(note, note_off)
            })
            .collect();

        self.played_notes.append(&mut played_notes);
    }

    pub fn output_note_off_events(&mut self, cycle: &Cycle, ticks_elapsed: &f64, writer: &mut Writer) {
        self.played_notes.retain(|played_note| {
            let a = played_note.note_off >= *ticks_elapsed 
                && played_note.note_off < ticks_elapsed + cycle.ticks_in_cycle;

            if a {
                let ticks_till_note = played_note.note_off - ticks_elapsed;
                let frames_till_note = ticks_till_note / cycle.ticks_in_cycle * cycle.frames as f64;

                // Don't round this as rounding error can cause frame to go over 256
                writer.write(played_note.note.note_off(frames_till_note.round() as u32));
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
                length: 4.0 * TICKS_PER_BEAT,
                played_notes: Vec::new(),
                notes: vec![
                    Note::new(0.0, 1.0 * TICKS_PER_BEAT, 74),
                    Note::new(TICKS_PER_BEAT * 1.0, 1.0 * TICKS_PER_BEAT, 69),
                    Note::new(TICKS_PER_BEAT * 2.0, 1.0 * TICKS_PER_BEAT, 69),
                    Note::new(TICKS_PER_BEAT * 3.0, 1.0 * TICKS_PER_BEAT, 69),
                ],
            },
        }
    }

    pub fn update_ticks(&mut self, cycle: &Cycle) {
        self.ticks_elapsed += cycle.ticks_in_cycle;
    }

    // This is only called when transport is running
    pub fn output_midi_note_on(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.pattern.output_note_on_events_in_cycle(cycle, &self.ticks_elapsed, writer);
    }

    // This is always called, also when transport is not running
    pub fn output_midi_note_off(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.pattern.output_note_off_events(cycle, &self.ticks_elapsed, writer);
    }
}
