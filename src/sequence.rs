
use super::{BEATS_PER_BAR, TICKS_PER_BEAT};
use super::message::Message;
use super::instrument::Instrument;
use super::grid::Grid;

#[derive(Debug)]
pub struct Sequence {
    // Phrase that's playing for instrument, array index = instrument
    phrases: [Option<usize>; 16],
    active: [bool; 16],

    pub active_grid: Grid,
    pub main_grid: Grid,
}

impl Sequence {
    fn create(phrases: [Option<usize>; 16]) -> Self {
        Sequence {
            phrases,
            active: [true; 16],

            active_grid: Grid::new(8, 1, 0x32),
            main_grid: Grid::new(8, 5, 0x35),
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

    pub fn active_phrases<'a>(&'a self) -> impl Iterator<Item=(usize, usize)> + 'a {
        self.phrases.iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                (instrument, phrase.unwrap())
            })
    }

    // Get bars of sequence based on the longest phrase it's playing
    pub fn bars(&self, instruments: &[Instrument; 16]) -> Option<u8> {
        self.active_phrases()
            .map(|(instrument, phrase)| {
                instruments[instrument].phrases[phrase].playable.bars
            })
            .max()
    }

    pub fn ticks(&self, instruments: &[Instrument; 16]) -> Option<u32> {
        self.bars(instruments)
            .and_then(|bars| Some(bars as u32 * BEATS_PER_BAR as u32 * TICKS_PER_BEAT as u32))
    }

    pub fn draw_sequence(&mut self, group: u8) -> Vec<Message> {
        let grid = &mut self.main_grid;
        let start = grid.width * group;
        let end = start + grid.width;

        self.phrases[start as usize .. end as usize].iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                grid.switch_led(instrument as u8, phrase.unwrap() as u8, 1)
            })
            .collect()
    }

    pub fn draw_active_grid(&mut self, group: u8) -> Vec<Message> {
        let leds = self.active_grid.width;

        (0..leds)
            .map(|led| {
                let is_active = self.active[(led + group * 8) as usize];
                let state = if is_active { 1 } else { 0 };
                self.active_grid.switch_led(led, 0, state)
            })
            .collect()
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
