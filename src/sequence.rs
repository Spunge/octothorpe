
use super::phrase::Phrase;
use super::instrument::Instrument;

#[derive(Clone, Copy)]
struct Play {
    phrase: u8,
    instrument: u8,
}

pub struct Sequence {
    plays: [Option<Play>; 16],
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            plays: [None; 16],
        }
    }

    pub fn play(instrument: u8, phrase: u8) {
    }
}
