
use super::TICKS_PER_BEAT;
use super::note::{Note, NoteOff};
use super::message::{Message, TimedMessage};
use super::cycle::Cycle;
use super::grid::Grid;

pub struct Pattern {
    bars: u8,
    beats_per_bar: u32,

    zoom_level: u32,
    zoom_offset: u8,

    notes: Vec<Note>,

    pattern_grid: Grid,
    length_grid: Grid,
    zoom_grid: Grid,
}

impl Pattern {
    fn create(notes: Vec<Note>) -> Self {
        Pattern {
            bars: 1,
            beats_per_bar: 4,

            zoom_level: 1,
            zoom_offset: 0,

            pattern_grid: Grid::new(8, 5, 0x35),
            length_grid: Grid::new(8, 1, 0x32),
            zoom_grid: Grid::new(8, 1, 0x31),

            notes,
        }
    }

    pub fn new() -> Self {
        Pattern::create(vec![])
    }

    pub fn default() -> Self {
        let ticks = TICKS_PER_BEAT as u32;
        let notes = vec![
            Note::new(0, ticks, 72, 127),
            Note::new(ticks, ticks, 69, 127),
            Note::new(ticks * 2, ticks, 69, 127),
            Note::new(ticks * 3, ticks, 69, 127),
        ];

        Pattern::create(notes)
    }

    pub fn alternate_default() -> Self {
        let ticks = TICKS_PER_BEAT as u32;
        let notes = vec![
            Note::new(0, ticks, 69, 127),
            Note::new(ticks, ticks, 71, 127),
            Note::new(ticks * 2, ticks, 69, 127),
            Note::new(ticks * 3, ticks, 73, 127),
        ];

        Pattern::create(notes)
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
        let grid = &mut self.pattern_grid;

        self.notes.iter()
            .filter_map(|note| {
                // TODO - mark all leds in length of note
                let x = note.tick / TICKS_PER_BEAT as u32 * 2;
                // Use A4 (69 in midi) as base note
                let y = 69 - note.key as i32;

                // Add 4 to push grid 4 down
                grid.try_switch_led(x as i32, y + 4, 1)
            })
            .collect()
    }
    
    pub fn draw_length(&mut self) -> Vec<Message> {
        (0..self.bars).map(|x| { self.length_grid.switch_led(x, 0, 1) }).collect()
    }

    pub fn draw_zoom(&mut self) -> Vec<Message> {
        let divide_by = 2_u8.pow(self.zoom_level);

        (0..(8 / divide_by)).map(|x| { self.zoom_grid.switch_led(x, 0, 1) }).collect()
    }

    pub fn note_on_messages(&self, cycle: &Cycle, channel: u8, offset: u32, interval: u32, note_offs: &mut Vec<NoteOff>) -> Vec<TimedMessage> {
        // Clone so we can change the tick on notes for next pattern iteration
        self.notes.iter()
            // Pattern could contain notes that fall not within start & finish of pattern
            .filter(|note| { note.tick < self.bars as u32 * self.beats_per_bar * TICKS_PER_BEAT as u32 })
            // It, is, play it, queing note off
            .filter_map(|note| {
                match cycle.delta_ticks_recurring(note.tick + offset, interval) {
                    Some(delta_ticks) => {
                        note_offs.push(note.note_off(cycle.absolute_start + delta_ticks));

                        Some(TimedMessage::new(cycle.ticks_to_frames(delta_ticks), note.message(channel)))

                    },
                    None => None,
                }
            })
            .collect()
    }
}

