
use super::message::Message;
use super::phrase::Phrase;
use super::instrument::Instrument;
use super::grid::Grid;

pub struct Sequence {
    // Phrase that's playing for instrument, array index = instrument
    plays: [Option<u8>; 16],
    active: [bool; 16],

    pub active_grid: Grid,
    pub main_grid: Grid,
}

impl Sequence {
    fn create(plays: [Option<u8>; 16]) -> Self {
        Sequence {
            plays,
            active: [true; 16],

            active_grid: Grid::new(8, 1, 0x32),
            main_grid: Grid::new(8, 5, 0x35),
        }
    }

    pub fn new() -> Self {
        Sequence::create([None; 16])
    }

    pub fn default() -> Self {
        let mut plays = [None; 16];
        plays[0] = Some(0);
        plays[1] = Some(0);

        Sequence::create(plays)
    }

    pub fn draw_sequence(&mut self, group: u8) -> Vec<Message> {
        let grid = &mut self.main_grid;
        let start = grid.width * group;
        let end = start + grid.width;

        self.plays[start as usize .. end as usize]
            .iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                grid.switch_led(instrument as u8, phrase.unwrap(), 1)
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

    pub fn play(instrument: u8, phrase: u8) {
    }
}
