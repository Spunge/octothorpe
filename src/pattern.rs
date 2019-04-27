
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
    pub fn default() -> Self {
        let ticks = TICKS_PER_BEAT as u32;

        Pattern {
            start: 0,
            length: 4,

            note_offs: Vec::new(),
            notes: vec![
                Note::new(0, ticks, 0, 74, 127),
                Note::new(ticks, ticks, 0, 69, 127),
                Note::new(ticks * 2, ticks, 0, 69, 127),
                Note::new(ticks * 3, ticks, 0, 69, 127),
            ],
        }
    }

    pub fn ticks(&self) -> u32 {
        self.length * TICKS_PER_BEAT as u32
    }

    pub fn output_note_on_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        let ticks = self.ticks();
        let note_offs = &mut self.note_offs;

        // Clone so we can change the tick on notes for next pattern iteration
        self.notes.iter()
            // Is note located within pattern?
            .filter(|note| { note.tick < ticks })
            // It, is, play it, queing note off
            .for_each(|note| {
                if let Some(delta_ticks) = cycle.delta_ticks_recurring(note.tick, ticks) {
                    // Write note
                    writer.write(note.note_on(cycle.ticks_to_frames(delta_ticks)));

                    // Absolute tick note_off should be tiggered
                    let new = NoteOff::new(*note, cycle.absolute_start + delta_ticks + note.length);

                    note_offs.retain(|old| {;
                        old.note.key != new.note.key
                    });

                    note_offs.push(new);
                }
            });
    }
    
    pub fn output_note_off_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    writer.write(note_off.note.note_off(frames));
                    false
                },
                None => true
            }
        });
    }
}

