
use super::{BEATS_PER_BAR, TICKS_PER_BEAT};
use super::pattern::Pattern;
use super::phrase::{Phrase, Play};
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::note::NoteOff;

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],
    note_offs: Vec<NoteOff>,
    
    pub phrase: usize,
    pub pattern: usize,
}

impl Instrument {
    pub fn new(c: u8) -> Self {
        let patterns = [ Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), ];
        let phrases = [ Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), ];

        Instrument {
            phrases,
            patterns,

            phrase: 0,
            pattern: 0,

            note_offs: vec![],
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern]
    }

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase]
    }

    pub fn note_off_messages(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut timed_messages = vec![];

        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    timed_messages.push(TimedMessage::new(frames, note_off.message()));
                    false
                },
                None => true
            }
        });

        timed_messages
    }

    pub fn note_on_messages(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        vec![]
        /*
         * TODO - Fighting th borrow checker as always..., but we're going to change this logic and move it to sequence anyway
        if cycle.is_rolling {

            let mut note_offs = vec![];
            let ticks_per_bar = BEATS_PER_BAR as u32 * TICKS_PER_BEAT as u32;
            let ticks = self.phrase().playable.bars as u32 * ticks_per_bar;
            let bars = self.phrase().playable.bars as u32;

            let messages = self.phrase().plays.iter()
                // Is play located within phrase?
                .filter(|play| { play.bar < bars })
                // Play pattern
                .map(|play| -> Vec<TimedMessage> {
                    let pattern = &self.patterns[play.pattern];

                    // Clone so we can change the tick on notes for next pattern iteration
                    pattern.notes.iter()
                        // Pattern could contain notes that fall not within start & finish of pattern
                        .filter(|note| { note.tick < pattern.playable.bars as u32 * BEATS_PER_BAR as u32 * TICKS_PER_BEAT as u32 })
                        // It, is, play it, queing note off
                        .filter_map(|note| {
                            match cycle.delta_ticks_recurring(note.tick + play.bar * ticks_per_bar, ticks) {
                                Some(delta_ticks) => {
                                    note_offs.push(note.note_off(cycle.absolute_start + delta_ticks));

                                    Some(TimedMessage::new(cycle.ticks_to_frames(delta_ticks), note.message()))

                                },
                                None => None,
                            }
                        })
                        .collect()
                })
                .flatten()
                .collect();

            self.note_offs.extend(note_offs);

            // Remove first occurences of same key notes
            self.note_offs.sort();
            self.note_offs.reverse();
            self.note_offs.dedup_by(|a, b| { a.key == b.key });

            messages
        } else {
            vec![]
        }
        */
    }
}
