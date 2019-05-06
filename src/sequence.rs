
use super::instrument::Instrument;

#[derive(Debug)]
pub struct Sequence {
    // Phrase that's playing for instrument, array index = instrument
    phrases: [Option<usize>; 16],
    pub active: [bool; 16],
}

impl Sequence {
    fn create(phrases: [Option<usize>; 16]) -> Self {
        Sequence {
            phrases,
            active: [true; 16],
        }
    }

    pub fn new() -> Self {
        Sequence::create([None; 16])
    }

    pub fn default() -> Self {
        let mut phrases = [None; 16];
        phrases[0] = Some(0);
        phrases[1] = Some(0);

        Sequence::create(phrases)
    }

    pub fn alternate_default() -> Self {
        let mut phrases = [None; 16];
        phrases[0] = Some(1);
        phrases[1] = Some(1);

        Sequence::create(phrases)
    }

    pub fn led_states(&mut self, group: u8) -> Vec<(i32, i32, u8)> {
        let start = 8 * group;
        let end = start + 8;

        self.phrases[start as usize .. end as usize].iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                (instrument as i32, phrase.unwrap() as i32, 1)
            })
            .collect()
    }

    pub fn active_phrases<'a>(&'a self) -> impl Iterator<Item=(usize, usize)> + 'a {
        self.phrases.iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                (instrument, phrase.unwrap())
            })
    }

    // Get bars of sequence based on the longest phrase it's playing
    pub fn ticks(&self, instruments: &[Instrument; 16]) -> Option<u32> {
        self.active_phrases()
            .map(|(instrument, phrase)| {
                instruments[instrument].phrases[phrase].playable.ticks
            })
            .max()
    }

    pub fn toggle_phrase(&mut self, instrument: u8, phrase: u8) {
        self.phrases[instrument as usize] = if let Some(old_phrase) = self.phrases[instrument as usize] {
            if old_phrase == phrase as usize {
                None
            } else {
                Some(phrase as usize)
            }
        } else {
            Some(phrase as usize)
        }
    }

    pub fn toggle_active(&mut self, instrument: u8) {
        self.active[instrument as usize] = ! self.active[instrument as usize];
    }

    pub fn playing_phrases(&self) -> Vec<(usize, usize)> {
        self.active_phrases()
            .filter(|(instrument, _)| self.active[*instrument])
            .collect()
    }
}
