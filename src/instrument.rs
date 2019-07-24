
use super::pattern::Pattern;
use super::phrase::Phrase;

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],
    pub knobs: [u8; 64],

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
            knobs: [0; 64],

            phrase: 0,
            pattern: 0,
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern]
    }

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase]
    }
}
