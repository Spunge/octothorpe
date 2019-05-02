
use std::ops::Range;

use super::{BEATS_PER_BAR, TICKS_PER_BEAT};
use super::note::Note;
use super::message::Message;
use super::playable::Playable;
use super::sequencer::KeyPress;

pub struct Pattern {
    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
    base_note: u8,
}

impl Pattern {
    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            playable: Playable::new(1, 1),
            channel,
            notes,
            // TODO - Use scales for this
            // Put a4 in center of grid
            base_note: 71,
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

    pub fn change_base_note(&mut self, delta: i32) -> bool {
        let base_note = self.base_note as i32 + delta;

        // 21 is A0
        if base_note >= 25 && base_note <= 127 {
            self.base_note = base_note as u8;
            true
        } else {
            false
        }
    }

    fn grid_measurements(&mut self) -> (u32, u32) {
        let led_ticks = (TICKS_PER_BEAT / 2.0) as u32 / self.playable.zoom * self.playable.bars as u32;
        let offset = self.playable.main_grid.width as u32 * self.playable.offset * led_ticks;
        (led_ticks, offset)
    }

    pub fn toggle_note(&mut self, x: Range<u8>, y: u8) -> Vec<Message> {
        let (led_ticks, offset) = self.grid_measurements();
        let start_tick = offset + led_ticks * x.start as u32;
        let end_tick = offset + led_ticks * (x.end + 1) as u32;

        let key = self.base_note - y;
        // TODO Velocity

        let notes = self.notes.len();
        
        self.notes.retain(|note| {
            (note.tick < start_tick || note.tick >= end_tick) || note.key != key
        });

        // No notes were removed, add new note, when note is longer as 1, the 1 note from the
        // previous keypress is removed, so ignore that
        if notes == self.notes.len() || x.start != x.end {
            self.notes.push(Note::new(self.channel, start_tick, end_tick - start_tick, key, 127));
        }

        let mut messages = self.playable.main_grid.clear(false);
        messages.extend(self.draw_pattern());
        messages
    }

    pub fn draw_pattern(&mut self) -> Vec<Message> {
        let (led_ticks, offset) = self.grid_measurements();
        let grid = &mut self.playable.main_grid;
        let base_note = self.base_note;

        self.notes.iter()
            .flat_map(|note| {
                // TODO - mark all leds in length of note
                let x = (note.tick as i32 - offset as i32) / led_ticks as i32;
                let y = base_note as i32 - note.key as i32;
            
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

