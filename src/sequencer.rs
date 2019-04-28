
use super::handlers::Writer;
use super::cycle::Cycle;
use super::instrument::Instrument;
use super::TICKS_PER_BEAT;
use super::grid::Grid;

pub enum View {
    Pattern,
    Phrase,
}

pub struct Sequencer {
    instruments: Vec<Instrument>,
    active_instrument: usize,
    view: View,
    instrument_grid: Grid,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut instruments = vec![Instrument::default(0)];
        instruments.append(&mut (1..16).map(|channel| { Instrument::new(channel) }).collect());

        Sequencer{
            instruments,
            active_instrument: 0,
            view: View::Pattern,
            instrument_grid: Grid::new(8, 1, 0x33),
        }
    }

    pub fn active_instrument(&mut self) -> &mut Instrument {
        &mut self.instruments[self.active_instrument]
    }

    fn draw_instruments(&mut self, frame: u32, writer: &mut Writer) {
        let led = self.active_instrument as u8 % self.instrument_grid.width;

        self.instrument_grid.switch_led(led, 0, 1, frame, writer);
    }
    
    // Called on start
    pub fn draw(&mut self, frame: u32, writer: &mut Writer) {
        self.draw_instruments(frame, writer);

        match self.view {
            View::Pattern => {
                let pattern = self.active_instrument().active_pattern();

                pattern.draw_pattern(frame, writer);
                pattern.draw_length(frame, writer);
                pattern.draw_zoom(frame, writer);
            },
            View::Phrase => { },
        };
    }

    pub fn draw_dynamic(&mut self, cycle: &Cycle, writer: &mut Writer) {
        match self.view {
            View::Pattern => {
                let pattern = self.active_instrument().active_pattern();

                if cycle.was_repositioned {
                    let beat_start = (cycle.start / TICKS_PER_BEAT as u32) * TICKS_PER_BEAT as u32;
                    let reposition_cycle = cycle.repositioned(beat_start);

                    pattern.draw_indicator(&reposition_cycle, writer);
                }

                // Update grid when running, after repositioning
                if cycle.is_rolling {
                    pattern.draw_indicator(cycle, writer);
                }
            },
            View::Phrase => { },
        }
    }

    pub fn output(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.instruments.iter_mut().for_each(|instrument| {
            instrument.output(cycle, writer);
        });
    }
}
