
use super::loopable::*;

#[derive(Debug)]
pub struct RecordedMessage {
    time: u32,
    channel: u8,
    key: u8,
    velocity: u8,
}

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],

    pub phrase: usize,
    pub pattern: usize,

    pub knob_group: u8,
    knob_values: [u8; 128],

    pub recorded_messages: Vec<RecordedMessage>,
    pub quantize_level: u8,
}

impl Instrument {
    pub fn new(c: u8) -> Self {
        let patterns = [ Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new(), Pattern::new(), ];
        let phrases = [ Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), ];

        Instrument {
            phrases,
            patterns,
            phrase: 0,
            pattern: 0,

            // There's 4 knob groups, this way we can have knobs * 4 !
            knob_group: 0,
            knob_values: [0; 128],

            recorded_messages: vec![],
            quantize_level: 2,
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern]
    }

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase]
    }

    pub fn get_pattern(&mut self, index: u8) -> &mut Pattern {
        &mut self.patterns[index as usize]
    }

    pub fn get_phrase(&mut self, index: u8) -> &mut Phrase {
        &mut self.phrases[index as usize]
    }

    pub fn clone_pattern(&mut self, from: u8, to: u8) {
        self.patterns[to as usize] = self.patterns[from as usize].clone();
    }

    pub fn clone_phrase(&mut self, from: u8, to: u8) {
        self.phrases[to as usize] = self.phrases[from as usize].clone();
    }

    pub fn switch_knob_group(&mut self, group: u8) {
        self.knob_group = group;
    }

    pub fn set_knob_value(&mut self, index: u8, value: u8) -> u8 {
        let knob = self.knob_group * 16 + index;
        self.knob_values[knob as usize] = value;
        knob
    }

    pub fn get_knob_values(&self) -> &[u8] {
        let start = self.knob_group as usize * 16;
        let end = start as usize + 16;
        &self.knob_values[start .. end]
    }

    pub fn knob_value_changed(&mut self, knob: u8, value: u8) -> Option<u8> {
        if self.knob_values[knob as usize] != value {
            self.knob_values[knob as usize] = value;
            Some(value)
        } else {
            None
        }
    }
}
