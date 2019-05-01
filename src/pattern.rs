
use super::{TICKS_PER_BEAT, BEATS_PER_BAR};
use super::note::{Note, NoteOff};
use super::message::{Message, TimedMessage};
use super::cycle::Cycle;
use super::playable::Playable;

pub struct Pattern {
    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
}

impl Pattern {
    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            playable: Playable::new(1, 1),
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
            Note::new(channel, 0, ticks, 73, 127),
            Note::new(channel, ticks, ticks, 69, 127),
            Note::new(channel, ticks * 2, ticks, 69, 127),
            Note::new(channel, ticks * 3, ticks, 69, 127),
        ];

        Pattern::create(channel, notes)
    }

    pub fn alternate_default(channel: u8) -> Self {
        let ticks = TICKS_PER_BEAT as u32;
        let offset = ticks / 2;
        let notes = vec![
            Note::new(channel, 0 + offset, ticks / 2, 71, 127),
            Note::new(channel, ticks + offset, ticks / 2, 70, 127),
            Note::new(channel, ticks * 2 + offset, ticks / 2, 72, 127),
            Note::new(channel, ticks * 3 + offset, ticks / 2, 70, 127),
        ];

        Pattern::create(channel, notes)
    }

    pub fn draw_pattern(&mut self) -> Vec<Message> {
        //let start_tick = 0;
        let led_ticks = (TICKS_PER_BEAT / 2.0) as u32 / self.playable.zoom * self.playable.bars as u32;
        let offset = self.playable.main_grid.width as u32 * self.playable.offset;
        let grid = &mut self.playable.main_grid;

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

    pub fn redraw(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        messages.extend(self.draw());
        messages
    }

    pub fn draw(&mut self) -> Vec<Message> {
        vec![ 
            self.draw_pattern(),
            self.playable.draw_length(),
            self.playable.draw_zoom() 
        ].into_iter().flatten().collect()
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        vec![ 
            self.playable.main_grid.clear(force), 
            self.playable.length_grid.clear(force),
            self.playable.zoom_grid.clear(force) 
        ].into_iter().flatten().collect()
    }
}

