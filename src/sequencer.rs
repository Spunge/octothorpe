
use super::TICKS_PER_BEAT;
use super::handlers::Writer;
use super::message::{Message, MessageData};
use super::cycle::Cycle;

#[derive(Debug, Clone, Copy)]
struct Note {
    // Ticks in pattern that note should be played
    pub tick: u32,
    pub length: u32,

    key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    fn new(tick: u32, length: u32, key: u8) -> Self {
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
    note_off: u32,
}

impl PlayedNote {
    fn new(note: Note, note_off: u32) -> Self {
        PlayedNote { note, note_off }
    }
}

#[derive(Debug)]
pub struct Pattern {
    start: u32,
    length: u32,
    notes: Vec<Note>,
    played_notes: Vec<PlayedNote>,
}

impl Pattern {
    fn ticks(&self) -> u32 {
        self.length * TICKS_PER_BEAT as u32
    }

    pub fn output_note_on_events(&mut self, pattern_cycle: &Cycle, absolute_cycle: &Cycle, writer: &mut Writer) {
        // Clone so we can change the tick on notes for next pattern iteration
        let mut played_notes = self.notes.iter()
            .cloned()
            // If note in next iteration of the pattern does belong in this cycle, add it
            .map(|mut note| {
                if pattern_cycle.contains(note.tick + self.ticks()) {
                    note.tick += self.ticks();
                }
                note
            })
            // Check all notes to see if they belong in this cycle
            .filter(|note| {
                pattern_cycle.contains(note.tick)
            })
            // Play notes
            .map(|note| {
                // Write note
                writer.write(note.note_on(pattern_cycle.frames_till_tick(note.tick)));

                PlayedNote::new(note, absolute_cycle.start + pattern_cycle.ticks_till_tick(note.tick + note.length))
            })
            .collect();

        // TODO - When note is pushed that is already in the list, we need to remove it as MIDI
        // will cut off that note
        self.played_notes.append(&mut played_notes);
    }

    pub fn output_note_off_events(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.played_notes.retain(|played_note| {
            if cycle.contains(played_note.note_off) {
                writer.write(played_note.note.note_off(cycle.frames_till_tick(played_note.note_off)));
            }

            // Return the opposite of A to keep notes that are not yet finished
            !cycle.contains(played_note.note_off)
        });
    }
}

pub struct Indicator {
    active_led: u32,
}

impl Indicator {
    fn activate_led(&mut self, cycle: &Cycle, led: u32, writer: &mut Writer) {
        println!("{:?}", cycle);
        println!("{}", led);
    }

    fn output(&mut self, cycle: &Cycle, pattern: &Pattern, writer: &mut Writer) {
        (0..pattern.length)
            .filter(|beat| {
                cycle.contains(beat * TICKS_PER_BEAT as u32)
            })
            .for_each(|beat| {
                self.activate_led(cycle, beat, writer);
            });
    }
}

pub struct Sequencer {
    pattern: Pattern,
    indicator: Indicator,
    // Keep track of elapsed ticks to trigger note_off when transport stops
    ticks_elapsed: u32,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            ticks_elapsed: 0,

            indicator: Indicator{
                active_led: 0,
            },

            pattern: Pattern {
                start: 0,
                length: 8,

                played_notes: Vec::new(),
                notes: vec![
                    Note::new(0, TICKS_PER_BEAT as u32, 72),
                    Note::new(TICKS_PER_BEAT as u32, TICKS_PER_BEAT as u32, 69),
                    Note::new(TICKS_PER_BEAT as u32 * 2, TICKS_PER_BEAT as u32, 69),
                    Note::new(TICKS_PER_BEAT as u32 * 3, TICKS_PER_BEAT as u32, 69),
                ],
            },
        }
    }

    pub fn output(&mut self, cycle: Cycle, control_out: &mut Writer, midi_out: &mut Writer) {
        let pattern_cycle = cycle.repositioned(cycle.start % self.pattern.ticks());
        let absolute_cycle = cycle.repositioned(self.ticks_elapsed);

        // Always turn notes off after their time is up to prevent infinite notes
        self.pattern.output_note_off_events(&absolute_cycle, midi_out);

        if cycle.is_rolling {
            self.indicator.output(&pattern_cycle, &self.pattern, control_out);
            self.pattern.output_note_on_events(&pattern_cycle, &absolute_cycle, midi_out);
        }

        // Update next ticks to keep track of absoulute ticks elapsed for note off events
        self.ticks_elapsed += cycle.ticks;
    }
}
