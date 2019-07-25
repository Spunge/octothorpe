
use super::pattern::Pattern;
use super::phrase::Phrase;

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],

    pub phrase: usize,
    pub pattern: usize,

    pub knob_group: u8,
    knob_values: [u8; 64],
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

            // There's 4 knob groups, this way we can have knobs * 4 !
            knob_group: 0,
            knob_values: [0; 64],
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern]
    }

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase]
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
