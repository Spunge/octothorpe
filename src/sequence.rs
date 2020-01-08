
use super::instrument::Instrument;

pub struct Sequence {
    // Phrase that's playing for instrument, array index = instrument
    phrases: [Option<u8>; 16],
    active: [bool; 16],

    knob_group: u8,
    knob_values: [u8; 128],
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            phrases: [Some(0); 16],
            active: [true; 16],

            knob_group: 0,
            knob_values: [0; 128],
        }
    }

    pub fn phrases(&self) -> [Option<u8>; 16] {
        self.phrases
    }

    pub fn get_phrase(&self, instrument: usize) -> Option<u8> {
        self.phrases[instrument]
    }

    pub fn set_phrases(&mut self, phrase: u8) {
        self.phrases = [Some(phrase); 16];
    }

    pub fn set_phrase(&mut self, instrument: usize, phrase: u8) {
        self.phrases[instrument] = Some(phrase);
    }

    pub fn unset_phrase(&mut self, instrument: usize) {
        self.phrases[instrument] = None;
    }

    pub fn active_phrase(&self, instrument: usize) -> Option<u8> {
        self.phrases[instrument].and_then(|phrase| if self.active[instrument] { Some(phrase) } else { None })
    }

    pub fn toggle_active(&mut self, instrument: usize) {
        self.active[instrument as usize] = ! self.active[instrument as usize];
    }
}
