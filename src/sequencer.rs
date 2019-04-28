
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
    instrument_active: u8,
    instrument_group: u8,
    view: View,
    instrument_active_grid: Grid,
    instrument_group_grid: Grid,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut instruments = vec![Instrument::default(0)];
        instruments.append(&mut (1..16).map(|channel| { Instrument::new(channel) }).collect());

        Sequencer{
            instruments,
            instrument_active: 0,
            instrument_group: 0,
            view: View::Pattern,
            instrument_active_grid: Grid::new(8, 1, 0x33),
            instrument_group_grid: Grid::new(1, 1, 0x50),
        }
    }

    pub fn active_instrument(&mut self) -> &mut Instrument {
        &mut self.instruments[(self.instrument_group * 8 + self.instrument_active) as usize]
    }

    pub fn switch_instrument(&mut self, instrument: u8, writer: &mut Writer) {
        self.clear(0, writer);
        self.instrument_active = instrument;
        self.draw(0, writer);
    }

    pub fn switch_instrument_group(&mut self, writer: &mut Writer) {
        self.clear(0, writer);
        self.instrument_group = if self.instrument_group == 1 { 0 } else { 1 };
        self.draw(0, writer);
    }
    
    // Called on start
    pub fn draw(&mut self, frame: u32, writer: &mut Writer) {
        self.instrument_active_grid.switch_led(self.instrument_active, 0, 1, frame, writer);
        self.instrument_group_grid.switch_led(0, 0, self.instrument_group, frame, writer);

        match self.view {
            View::Pattern => { self.active_instrument().active_pattern().draw(frame, writer) },
            View::Phrase => { },
        };
    }

    pub fn clear(&mut self, frame: u32, writer: &mut Writer) {
        self.instrument_active_grid.clear_active(frame, writer);
        self.instrument_group_grid.clear_active(frame, writer);

        match self.view {
            View::Pattern => { self.active_instrument().active_pattern().clear(frame, writer) },
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
