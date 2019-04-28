
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
    instrument: u8,
    group: u8,
    view: View,

    indicator_grid: Grid,
    instrument_grid: Grid,
    group_grid: Grid,
    active_grid: Grid,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut instruments = vec![Instrument::default(0), Instrument::alternate_default(1)];
        instruments.append(&mut (2..16).map(|channel| { Instrument::new(channel) }).collect());

        Sequencer{
            instruments,
            instrument: 0,
            group: 0,
            view: View::Pattern,

            indicator_grid: Grid::new(8, 1, 0x34),
            instrument_grid: Grid::new(8, 1, 0x33),
            group_grid: Grid::new(1, 1, 0x50),
            active_grid: Grid::new(8, 1, 0x30),
        }
    }

    fn instrument_by_index(&mut self, index: u8) -> &mut Instrument {
        &mut self.instruments[(self.group * 8 + index) as usize]
    }

    pub fn instrument(&mut self) -> &mut Instrument {
        self.instrument_by_index(self.instrument)
    }

    pub fn switch_instrument(&mut self, instrument: u8, writer: &mut Writer) {
        self.clear(0, false, writer);
        self.instrument = instrument;
        self.draw(0, writer);
    }

    pub fn toggle_instrument_active(&mut self, instrument: u8, writer: &mut Writer) {
        self.active_grid.clear(0, false, writer);
        self.instrument_by_index(instrument).toggle_active();
        self.active_grid.clear(0, false, writer);
    }

    pub fn switch_group(&mut self, writer: &mut Writer) {
        self.clear(0, false, writer);
        self.group = if self.group == 1 { 0 } else { 1 };
        self.draw(0, writer);
    }

    pub fn draw_active_grid

    // Called on start
    pub fn draw(&mut self, frame: u32, writer: &mut Writer) {
        self.instrument_grid.switch_led(self.instrument, 0, 1, frame, writer);
        self.group_grid.switch_led(0, 0, self.group, frame, writer);

        match self.view {
            View::Pattern => { self.instrument().pattern().draw(frame, writer) },
            View::Phrase => { },
        };
    }

    pub fn clear(&mut self, frame: u32, force: bool, writer: &mut Writer) {
        self.instrument_grid.clear(frame, force, writer);
        self.group_grid.clear(frame, force, writer);

        match self.view {
            View::Pattern => { self.instrument().pattern().clear(frame, force, writer) },
            View::Phrase => { },
        };
    }

    pub fn draw_indicator(&mut self, cycle: &Cycle, writer: &mut Writer) {
        // TODO - Show 1 bar pattern over the whole grid, doubling the steps
        let steps = 8;
        let ticks = steps * TICKS_PER_BEAT as u32 / 2;

        (0..steps).for_each(|beat| { 
            let tick = beat * TICKS_PER_BEAT as u32 / 2;

            if let Some(delta_ticks) = cycle.delta_ticks_recurring(tick, ticks) {
                let frame = cycle.ticks_to_frames(delta_ticks);
                self.indicator_grid.clear(frame, false, writer);
                self.indicator_grid.try_switch_led(beat as i32, 0, 1, frame, writer)
            }
        })
    }

    pub fn draw_dynamic(&mut self, cycle: &Cycle, writer: &mut Writer) {
        match self.view {
            View::Pattern => {
                if cycle.was_repositioned {
                    let beat_start = (cycle.start / TICKS_PER_BEAT as u32) * TICKS_PER_BEAT as u32;
                    let reposition_cycle = cycle.repositioned(beat_start);

                    self.draw_indicator(&reposition_cycle, writer);
                }

                // Update grid when running, after repositioning
                if cycle.is_rolling {
                    self.draw_indicator(cycle, writer);
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
