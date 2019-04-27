
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
        let ticks = self.ticks();

        // Clone so we can change the tick on notes for next pattern iteration
        let mut note_offs: Vec<NoteOff> = self.notes.iter()
            .filter_map(|note| {
                match cycle.delta_frames_recurring(note.tick, ticks) {
                    Some(frames) => {
                        // Write note
                        writer.write(note.note_on(frames));

                        // Absolute tick note_off should be tiggered
                        let note_off = cycle.absolute_start + cycle.frames_to_ticks(frames) + note.length;
                        // TODO - When note is pushed that is already in the list, we need to remove it as MIDI
                        Some(NoteOff::new(*note, note_off))
                    },
                    None => { None },
                }
            })
            .collect();

        self.note_offs.append(&mut note_offs);
    }

    pub fn output_note_off_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    writer.write(note_off.note.note_off(frames));
                    true
                },
                None => false
            }
        });
    }
}

