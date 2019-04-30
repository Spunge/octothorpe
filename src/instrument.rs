
use super::pattern::Pattern;
use super::phrase::Phrase;
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::note::NoteOff;

pub struct Instrument {
    pub is_active: bool,

    // TODO - this is public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    phrases: [Phrase; 5],
    note_offs: Vec<NoteOff>,

    playing_phrase: usize,
    showing_phrase: usize,
    showing_pattern: usize,
}

impl Instrument {
    pub fn new(c: u8) -> Self {
        let patterns = [ Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), ];
        let phrases = [ Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), ];

        Instrument {
            is_active: true,

            patterns,
            phrases,
            playing_phrase: 0,
            showing_phrase: 0,
            showing_pattern: 0,

            note_offs: vec![],
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.showing_pattern]
    }

    pub fn phrase(&self) -> &Phrase {
        &self.phrases[self.showing_phrase]
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
        if cycle.is_rolling && self.is_active {
            let mut note_offs = vec![];

            // Get note offs by playing note_ons
            let messages = self.phrase().note_on_messages(cycle, &self.patterns, &mut note_offs);
            
            self.note_offs.extend(note_offs);

            // Remove first occurences of same key notes
            self.note_offs.sort();
            self.note_offs.reverse();
            self.note_offs.dedup_by(|a, b| { a.key == b.key });

            messages
        } else {
            vec![]
        }
    }
}
