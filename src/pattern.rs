
use super::TICKS_PER_BEAT;
use super::note::{Note, NoteOff};
use super::handlers::Writer;
use super::cycle::Cycle;

#[derive(Debug)]
pub struct Pattern {
    pub start: u32,
    pub length: u32,
    pub notes: Vec<Note>,
    pub note_offs: Vec<NoteOff>,
}

impl Pattern {
    pub fn ticks(&self) -> u32 {
        self.length * TICKS_PER_BEAT as u32
    }

    pub fn output_note_on_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        // Clone so we can change the tick on notes for next pattern iteration
        let mut note_offs = self.notes.iter()
            .cloned()
            // Check all notes to see if they belong in this cycle
            .filter(|note| { cycle.contains_recurring(note.tick, self.ticks()) })
            // Play notes
            .map(|note| {
                // Write note
                writer.write(note.note_on(cycle.delta_frames_recurring(note.tick, self.ticks())));

                NoteOff::new(note, cycle.absolute_start + cycle.delta_ticks_recurring(note.tick, self.ticks()) + note.length)
            })
            .collect();

        // TODO - When note is pushed that is already in the list, we need to remove it as MIDI
        // will cut off that note
        self.note_offs.append(&mut note_offs);
    }

    pub fn output_note_off_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.note_offs.retain(|note_off| {
            if cycle.contains_absolute(note_off.tick) {
                writer.write(note_off.note.note_off(cycle.delta_frames_absolute(note_off.tick)));
            }

            // Return the opposite of A to keep notes that are not yet finished
            !cycle.contains_absolute(note_off.tick)
        });
    }
}

