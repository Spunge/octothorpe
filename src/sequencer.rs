
use super::TICKS_PER_BEAT;
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
use super::grid::Grid;
use super::instrument::Instrument;
use super::phrase::Phrase;
use super::pattern::Pattern;
use super::sequence::Sequence;
use super::playable::Playable;

pub enum OverView {
    Instrument,
    Sequence,
}

pub enum DetailView {
    Pattern,
    Phrase,
}

pub struct Sequencer {
    group: u8,

    instruments: [Instrument; 16],
    instrument: u8,

    sequences: [Sequence; 4],
    sequence: u8,

    // What is playing?
    playing_sequence: u8,
    queued_sequence: Option<u8>,

    // What are we showing?
    overview: OverView,
    detailview: DetailView,

    // Various indicators
    indicator_grid: Grid,
    instrument_grid: Grid,
    group_grid: Grid,
    playable_grid: Grid,
    detailview_grid: Grid,
    overview_grid: Grid,
}

impl Sequencer {
    pub fn new() -> Self {
        // Build instruments for each midi channel
        let mut instruments = [
            Instrument::new(0), Instrument::new(1), Instrument::new(2), Instrument::new(3),
            Instrument::new(4), Instrument::new(5), Instrument::new(6), Instrument::new(7),
            Instrument::new(8), Instrument::new(9), Instrument::new(10), Instrument::new(11),
            Instrument::new(12), Instrument::new(13), Instrument::new(14), Instrument::new(15),
        ];
        instruments[0].patterns[0] = Pattern::default(0);
        instruments[1].patterns[0] = Pattern::alternate_default(1);
        instruments[0].phrases[0] = Phrase::default();
        instruments[1].phrases[0] = Phrase::default();
    
        // Build sequence we can trigger
        let sequences = [ Sequence::new(), Sequence::new(), Sequence::new(), Sequence::new(), ];

        Sequencer{
            instruments,
            instrument: 0,
            group: 0,

            sequences,
            sequence: 0,

            playing_sequence: 0,
            queued_sequence: None,

            // What are we currently showing?
            detailview: DetailView::Pattern,
            overview: OverView::Instrument,

            // Only show in instrument overview
            playable_grid: Grid::new(1, 5, 0x52),
            instrument_grid: Grid::new(8, 1, 0x33),

            // Show in both overviews
            group_grid: Grid::new(1, 1, 0x50),
            detailview_grid: Grid::new(1, 1, 0x3E),
            overview_grid: Grid::new(1, 1, 0x3A),
            indicator_grid: Grid::new(8, 1, 0x34),
        }
    }

    fn instrument(&mut self) -> &mut Instrument {
        &mut self.instruments[(self.group * 8 + self.instrument) as usize]
    }

    fn sequence(&mut self) -> &mut Sequence {
        &mut self.sequences[self.sequence as usize]
    }

    fn instrument_key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
        match message.bytes[1] {
            0x3E => self.switch_detailview(),
            0x33 => self.switch_instrument(message.bytes[0] - 0x90),
            0x31 | 0x32 | 0x60 | 0x61 => {
                let should_redraw = match message.bytes[1] {
                    0x31 => self.playable().change_zoom((message.bytes[0] - 0x90 + 1) as u32),
                    0x32 => self.playable().change_length(message.bytes[0] - 0x90 + 1),
                    0x61 => self.playable().change_offset(-1),
                    0x60 => self.playable().change_offset(1),
                    _ => false,
                };

                if should_redraw { self.redraw_instrument() } else { vec![] }
            },
            _ => vec![],
        }
    }

    fn sequence_key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
        vec![]
    }

    pub fn key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
        match message.bytes[1] {
            //0x30 => self.sequencer.toggle_instrument_active(message.bytes[0] - 0x90),
            0x50 => self.switch_group(),
            0x3A => self.switch_overview(),
            // Stuff for instruments
            0x3E | 0x31 | 0x33 | 0x32 | 0x60 | 0x61 => {
                match self.overview {
                    OverView::Instrument => self.instrument_key_pressed(message),
                    OverView::Sequence => self.sequence_key_pressed(message),
                }
            }
            _ => vec![],
        }
    }

    fn switch_group(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        self.group = if self.group == 1 { 0 } else { 1 };
        messages.append(&mut self.draw());
        messages
    }

    fn switch_instrument(&mut self, instrument: u8) -> Vec<Message> {
        let mut messages = self.clear_instrument(false);
        self.instrument = instrument;
        messages.append(&mut self.draw_instrument());
        messages
    }

    fn switch_overview(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        match self.overview {
            OverView::Instrument => { self.overview = OverView::Sequence },
            OverView::Sequence => { self.overview = OverView::Instrument },
        }
        messages.append(&mut self.draw());
        messages
    }

    fn switch_detailview(&mut self) -> Vec<Message> {
        let mut messages = self.clear_instrument(false);
        match self.detailview {
            DetailView::Pattern => { self.detailview = DetailView::Phrase },
            DetailView::Phrase => { self.detailview = DetailView::Pattern },
        }
        messages.append(&mut self.draw_instrument());
        messages
    }

    fn playable(&mut self) -> &mut Playable {
        match self.detailview {
            DetailView::Pattern => { &mut self.instrument().pattern().playable },
            DetailView::Phrase => { &mut self.instrument().phrase().playable },
        }
    }

    fn playable_led(&mut self) -> usize {
        match self.detailview {
            DetailView::Pattern => self.instrument().pattern,
            DetailView::Phrase => self.instrument().phrase,
        }
    }

    fn draw_sequencer(&mut self) -> Vec<Message> {
        vec![
            self.group_grid.switch_led(0, 0, self.group),
            self.overview_grid.switch_led(0, 0, match self.overview { OverView::Instrument => 1, _ => 0, }),
        ]
    }

    fn clear_sequencer(&mut self, force: bool) -> Vec<Message> {
        vec![
            self.group_grid.clear(force),
            self.detailview_grid.clear(force),
            self.overview_grid.clear(force),
        ].into_iter().flatten().collect()
    }

    fn draw_instrument(&mut self) -> Vec<Message> {
        let playable_led = self.playable_led() as u8;
        // Only show detailview led when we're in instument view
        let detailview_led = match (&self.overview, &self.detailview) { 
            (OverView::Instrument, DetailView::Pattern) => 1, 
            _ => 0, 
        };

        vec![
            vec![
                self.instrument_grid.switch_led(self.instrument, 0, 1),
                self.playable_grid.switch_led(0, playable_led, 1),
                self.detailview_grid.switch_led(0, 0, detailview_led),
            ],
            match self.detailview {
                DetailView::Pattern => { self.instrument().pattern().draw() },
                DetailView::Phrase => { self.instrument().phrase().draw() },
            },
        ].into_iter().flatten().collect()
    }

    fn redraw_instrument(&mut self) -> Vec<Message> {
        match self.detailview {
            DetailView::Pattern => { 
                let mut messages = self.instrument().pattern().clear(false);
                messages.extend(self.instrument().pattern().draw());
                messages
            },
            DetailView::Phrase => { 
                let mut messages = self.instrument().phrase().clear(false);
                messages.extend(self.instrument().phrase().draw());
                messages
            },
        }
    }

    fn clear_instrument(&mut self, force: bool) -> Vec<Message> {
        vec![
            self.instrument_grid.clear(force),
            self.playable_grid.clear(force),
            self.detailview_grid.clear(force),
            match self.detailview {
                DetailView::Pattern => { self.instrument().pattern().clear(force) },
                DetailView::Phrase => { self.instrument().phrase().clear(force) },
            },
        ].into_iter().flatten().collect()
    }

    fn draw_sequence(&mut self) -> Vec<Message> {
        vec![]
    }

    fn clear_sequence(&mut self, force: bool) -> Vec<Message> {
        vec![]
    }
   
    fn draw_overview(&mut self) -> Vec<Message> {
        match self.overview {
            OverView::Instrument => self.draw_instrument(),
            OverView::Sequence => self.draw_sequence(),
        }
    }
    
    fn clear_overview(&mut self, force: bool) -> Vec<Message> {
        match self.overview {
            OverView::Instrument => self.clear_instrument(force),
            OverView::Sequence => self.clear_sequence(force),
        }
    }

    // Draw all the things
    pub fn draw(&mut self) -> Vec<Message> {
        let mut messages = self.draw_sequencer();
        messages.extend(self.draw_overview());
        messages
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        let mut messages = self.clear_sequencer(force);
        messages.extend(self.clear_overview(force));
        messages
    }

    fn draw_indicator(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
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
            match self.detailview {
                DetailView::Pattern => {
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
                DetailView::Phrase => None,
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
