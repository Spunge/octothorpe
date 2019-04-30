
use super::TICKS_PER_BEAT;
use super::note::{Note, NoteOff};
use super::message::{Message, TimedMessage};
use super::cycle::Cycle;
use super::grid::Grid;

pub struct Pattern {
    bars: u8,
    beats_per_bar: u32,

    zoom: u32,
    offset: u32,

    channel: u8,
    notes: Vec<Note>,

    pattern_grid: Grid,
    length_grid: Grid,
    zoom_grid: Grid,
}

impl Pattern {
    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            bars: 1,
            beats_per_bar: 4,

            zoom: 1, 
            offset: 0,

            pattern_grid: Grid::new(8, 5, 0x35),
            length_grid: Grid::new(8, 1, 0x32),
            zoom_grid: Grid::new(8, 1, 0x31),

            channel,
            notes,
        }
    }

    pub fn new(channel: u8) -> Self {
        Pattern::create(channel, vec![])
    }

    pub fn default(channel: u8) -> Self {
        let ticks = TICKS_PER_BEAT as u32;
        let notes = vec![
            Note::new(channel, 0, ticks, 72, 127),
            Note::new(channel, ticks, ticks, 69, 127),
            Note::new(channel, ticks * 2, ticks, 69, 127),
            Note::new(channel, ticks * 3, ticks, 69, 127),
        ];

        Pattern::create(channel, notes)
    }

    pub fn alternate_default(channel: u8) -> Self {
        let ticks = TICKS_PER_BEAT as u32;
        let offset = (TICKS_PER_BEAT * 0.5) as u32;
        let notes = vec![
            Note::new(channel, 0 + offset, ticks / 2, 71, 127),
            Note::new(channel, ticks + offset, ticks / 2, 72, 127),
            Note::new(channel, ticks * 2 + offset, ticks / 2, 71, 127),
            Note::new(channel, ticks * 3 + offset, ticks / 2, 72, 127),
        ];

        Pattern::create(channel, notes)
    }

    pub fn change_zoom(&mut self, button: u32) {
        match button {
            1 | 2 | 4 | 8 => { self.zoom = 8 / button; self.offset = 0; },
            5 => { self.zoom = 2; self.offset = 1; },
            7 => { self.zoom = 4; self.offset = 3; },
            3 | 6 => { self.zoom = 8; self.offset = button - 1; },
            _ => {},
        }
    }

    pub fn change_offset(&mut self, delta: i32) {
        let offset = self.offset as i32 + delta;

        if offset >= 0 && offset <= self.zoom as i32 - 1 {
            self.offset = offset as u32;
        }
    }
    
    pub fn change_length(&mut self, bars: u8) {
        if bars > 0 && bars <= 8 {
            self.bars = bars;
        }
    }

    pub fn redraw(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        messages.extend(self.draw());
        messages
    }

    pub fn draw(&mut self) -> Vec<Message> {
        vec![ self.draw_pattern(), self.draw_length(), self.draw_zoom() ].into_iter().flatten().collect()
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        vec![ 
            self.pattern_grid.clear(force), 
            self.length_grid.clear(force),
            self.zoom_grid.clear(force) 
        ].into_iter().flatten().collect()
    }

    pub fn draw_pattern(&mut self) -> Vec<Message> {
        //let start_tick = 0;
        let led_ticks = (TICKS_PER_BEAT / 2.0) as u32 / self.zoom;
        let offset = self.pattern_grid.width as u32 * self.offset;
        let grid = &mut self.pattern_grid;

        self.notes.iter()
            .flat_map(|note| {
                // TODO - mark all leds in length of note
                let absolute_led = note.tick as i32 / led_ticks as i32;
                let x = absolute_led as i32 - offset as i32;
                let y = 73 - note.key as i32;
            
                let mut leds = vec![ (x, y, 1) ];

                (1..(note.length / led_ticks)).for_each(|led| {
                    leds.push((x + led as i32, y, 5))
                });

                leds
            })
            .filter_map(|pos| {
                let (x, y, state) = pos;

                // Add 4 to push grid 4 down, 69 as base A4 in midi
                grid.try_switch_led(x, y, state)
            })
            .collect()
    }
    
    pub fn draw_length(&mut self) -> Vec<Message> {
        (0..self.bars).map(|x| { self.length_grid.switch_led(x, 0, 1) }).collect()
    }

    pub fn draw_zoom(&mut self) -> Vec<Message> {
        let length = 8 / self.zoom;
        let from = self.offset * length;
        let to = from + length;

        (from..to)
            .map(|x| { self.zoom_grid.switch_led(x as u8, 0, 1) })
            .collect()
    }

    pub fn note_on_messages(&self, cycle: &Cycle, offset: u32, interval: u32, note_offs: &mut Vec<NoteOff>) -> Vec<TimedMessage> {
        // Clone so we can change the tick on notes for next pattern iteration
        self.notes.iter()
            // Pattern could contain notes that fall not within start & finish of pattern
            .filter(|note| { note.tick < self.bars as u32 * self.beats_per_bar * TICKS_PER_BEAT as u32 })
            // It, is, play it, queing note off
            .filter_map(|note| {
                match cycle.delta_ticks_recurring(note.tick + offset, interval) {
                    Some(delta_ticks) => {
                        note_offs.push(note.note_off(cycle.absolute_start + delta_ticks));

                        Some(TimedMessage::new(cycle.ticks_to_frames(delta_ticks), note.message()))

                    },
                    None => None,
                }
            })
            .collect()
    }
}

