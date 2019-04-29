
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
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

    pub fn switch_instrument(&mut self, instrument: u8) -> Vec<Message> {
        let mut messages = self.clear(false);
        self.instrument = instrument;
        messages.append(&mut self.draw());
        messages
    }

    pub fn toggle_instrument_active(&mut self, instrument: u8) -> Vec<Message> {
        let instrument = self.instrument_by_index(instrument);
        instrument.is_active = ! instrument.is_active;
        self.draw_active_grid()
    }

    pub fn switch_group(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        self.group = if self.group == 1 { 0 } else { 1 };
        messages.append(&mut self.draw());
        messages
    }

    pub fn draw_active_grid(&mut self) -> Vec<Message> {
        let leds = self.active_grid.width;

        (0..leds)
            .map(|led| {
                let instrument = self.instrument_by_index(led);
                let state = if instrument.is_active { 1 } else { 0 };
                self.active_grid.switch_led(led, 0, state)
            })
            .collect()
    }

    // Called on start
    pub fn draw(&mut self) -> Vec<Message> {
        vec![
            vec![
                self.instrument_grid.switch_led(self.instrument, 0, 1),
                self.group_grid.switch_led(0, 0, self.group),
            ],
            self.draw_active_grid(),
            match self.view {
                View::Pattern => { self.instrument().pattern().draw() },
                View::Phrase => { vec![] },
            },
        ].into_iter().flatten().collect()
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        vec![
            self.instrument_grid.clear(force),
            self.group_grid.clear(force),
            match self.view {
                View::Pattern => { self.instrument().pattern().clear(force) },
                View::Phrase => { vec![] },
            },
        ].into_iter().flatten().collect()
    }

    pub fn draw_indicator(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        // TODO - Show 1 bar pattern over the whole grid, doubling the steps
        let steps = 8;
        let ticks = steps * TICKS_PER_BEAT as u32 / 2;

        (0..steps)
            .filter_map(|beat| { 
                let tick = beat * TICKS_PER_BEAT as u32 / 2;

                cycle.delta_ticks_recurring(tick, ticks)
                    .and_then(|delta_ticks| {
                        let mut messages = self.indicator_grid.clear(false);
                        if let Some(message) = self.indicator_grid.try_switch_led(beat as i32, 0, 1) {
                            messages.push(message);
                        }
    
                        let mut timed_messages = vec![];
    
                        let frame = cycle.ticks_to_frames(delta_ticks);
                        for message in messages.into_iter() {
                            timed_messages.push(TimedMessage::new(frame, message))
                        }

                        Some(timed_messages)
                    })
            })
            .flatten()
            .collect()
    }

    // TODO - Move this logic to indicator func
    pub fn draw_dynamic(&mut self, cycle: &Cycle) -> Option<Vec<TimedMessage>> {
        if cycle.was_repositioned || cycle.is_rolling {
            match self.view {
                View::Pattern => {
                    let mut messages = vec![];

                    if cycle.was_repositioned {
                        let beat_start = (cycle.start / TICKS_PER_BEAT as u32) * TICKS_PER_BEAT as u32;
                        let reposition_cycle = cycle.repositioned(beat_start);

                        messages.extend(self.draw_indicator(&reposition_cycle));
                    }

                    // Update grid when running, after repositioning
                    if cycle.is_rolling {
                        messages.extend(self.draw_indicator(cycle));
                    }

                    Some(messages)
                },
                View::Phrase => None,
            }
        } else {
            None
        }
    }

    pub fn output(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        self.instruments.iter_mut()
            .flat_map(|instrument| {
                let mut messages = instrument.note_off_messages(cycle);
                messages.extend(instrument.note_on_messages(cycle));
                messages
            })
            .collect()
    }
}
