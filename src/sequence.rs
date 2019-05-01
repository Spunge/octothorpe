
use super::message::Message;
use super::phrase::Phrase;
use super::instrument::Instrument;
use super::grid::Grid;

#[derive(Clone, Copy)]
struct Play {
    phrase: u8,
    instrument: u8,
}

pub struct Sequence {
    plays: [Option<Play>; 16],
    active_grid: Grid,
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            plays: [None; 16],

            active_grid: Grid::new(8, 1, 0x30),
        }
    }

    // TODO - Move this to sequence
    pub fn draw_active_grid(&mut self) -> Vec<Message> {
        vec![]
        /*
        let leds = self.active_grid.width;

        (0..leds)
            .map(|led| {
                let instrument = self.instrument_by_index(led);
                let state = if instrument.is_active { 1 } else { 0 };
                self.active_grid.switch_led(led, 0, state)
            })
            .collect()
            */
    }

    pub fn toggle_instrument_active(&mut self, instrument: u8) -> Vec<Message> {
        vec![]
        //let instrument = self.instrument_by_index(instrument);
        //instrument.is_active = ! instrument.is_active;
        //self.draw_active_grid()
    }

    pub fn play(instrument: u8, phrase: u8) {
    }
}
