
use super::pattern::Pattern;
use super::phrase::Phrase;
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::note::NoteOff;

pub struct Instrument {
    pub is_active: bool,
    channel: u8,

    patterns: Vec<Pattern>,
    phrases: Vec<Phrase>,
    note_offs: Vec<NoteOff>,

    playing_phrase: usize,
    showing_phrase: usize,
    showing_pattern: usize,
}

impl Instrument {
    fn create(channel: u8, patterns: Vec<Pattern>, phrases: Vec<Phrase>) -> Self {
        Instrument {
            is_active: true,

            patterns,
            phrases,
            playing_phrase: 0,
            showing_phrase: 0,
            showing_pattern: 0,

            note_offs: vec![],
            channel,
        }
    }
    
    pub fn new(channel: u8) -> Self {
        Instrument::create(channel, vec![Pattern::new()], vec![Phrase::new()])
    }

    pub fn default(channel: u8) -> Self {
        Instrument::create(channel, vec![Pattern::default()], vec![Phrase::default()]) 
    }

    pub fn alternate_default(channel: u8) -> Self {
        Instrument::create(channel, vec![Pattern::alternate_default()], vec![Phrase::default()]) 
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.showing_pattern]
    }

    pub fn phrase(&self) -> &Phrase {
        &self.phrases[self.showing_phrase]
    }

    pub fn note_off_messages(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut timed_messages = vec![];
        let channel = self.channel;

        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    timed_messages.push(TimedMessage::new(frames, note_off.message(channel)));
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
            let messages = self.phrase().note_on_messages(cycle, self.channel, &self.patterns, &mut note_offs);
            
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
