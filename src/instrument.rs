
use super::pattern::Pattern;
use super::phrase::Phrase;
use super::handlers::Writer;
use super::cycle::Cycle;
use super::note::NoteOff;

pub struct Instrument {
    pub is_active: bool,

    patterns: Vec<Pattern>,
    phrases: Vec<Phrase>,
    playing_phrase: usize,
    showing_phrase: usize,
    showing_pattern: usize,

    note_offs: Vec<NoteOff>,
    channel: u8,
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

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.showing_phrase]
    }

    pub fn output_note_offs(&mut self, cycle: &Cycle, writer: &mut Writer) {
        let channel = self.channel;

        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    writer.write(note_off.note.note_off(frames, channel));
                    false
                },
                None => true
            }
        });
    }

    // Output midi
    pub fn output(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.output_note_offs(cycle, writer);

        if cycle.is_rolling && self.is_active {
            // Get note offs by playing note_ons
            self.note_offs.append(&mut self.phrases[self.playing_phrase]
                                  .output_notes(cycle, self.channel, &self.patterns, writer));

            // Remove first occurences of same key notes
            self.note_offs.sort();
            self.note_offs.reverse();
            self.note_offs.dedup_by(|a, b| { a.note.key == b.note.key });
        }
    }
}
