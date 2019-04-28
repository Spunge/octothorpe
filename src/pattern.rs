
use super::TICKS_PER_BEAT;
use super::note::{Note, NoteOff};
use super::handlers::Writer;
use super::cycle::Cycle;
use super::grid::Grid;

pub struct Pattern {
    bars: u8,
    beats_per_bar: u32,

    zoom_level: u32,
    zoom_offset: u8,

    pub notes: Vec<Note>,

    pattern_grid: Grid,
    length_grid: Grid,
    indicator_grid: Grid,
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
            indicator_grid: Grid::new(8, 1, 0x34),
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

    pub fn draw(&mut self, frame: u32, writer: &mut Writer) {
        self.draw_pattern(frame, writer);
        self.draw_length(frame, writer);
        self.draw_zoom(frame, writer);
    }

    pub fn clear(&mut self, frame: u32, writer: &mut Writer) {
        self.pattern_grid.clear_active(frame, writer);
        self.length_grid.clear_active(frame, writer);
        self.zoom_grid.clear_active(frame, writer);
    }

    pub fn draw_pattern(&mut self, frame: u32, writer: &mut Writer) {
        let grid = &mut self.pattern_grid;

        self.notes.iter()
            .for_each(|note| {
                let x = note.tick / TICKS_PER_BEAT as u32 * 2;
                // Use A4 (69 in midi) as base note
                let y = 69 - note.key as i32;

                // Add 4 to push grid 4 down
                grid.try_switch_led(x as i32, y + 4, 1, frame, writer);
            });
    }
    
    pub fn draw_length(&mut self, frame: u32, writer: &mut Writer) {
        (0..self.bars).for_each(|x| {
            self.length_grid.switch_led(x, 0, 1, frame, writer);
        });
    }

    pub fn draw_zoom(&mut self, frame: u32, writer: &mut Writer) {
        let divide_by = 2_u8.pow(self.zoom_level);

        (0..(8 / divide_by)).for_each(|x| {
            self.zoom_grid.switch_led(x, 0, 1, frame, writer);
        })
    }

    pub fn draw_indicator(&mut self, cycle: &Cycle, writer: &mut Writer) {
        // TODO - Show 1 bar pattern over the whole grid, doubling the steps
        let steps = 8;
        let ticks = steps * TICKS_PER_BEAT as u32 / 2;

        (0..steps).for_each(|beat| { 
            let tick = beat * TICKS_PER_BEAT as u32 / 2;

            if let Some(delta_ticks) = cycle.delta_ticks_recurring(tick, ticks) {
                let frame = cycle.ticks_to_frames(delta_ticks);
                self.indicator_grid.clear_active(frame, writer);
                self.indicator_grid.try_switch_led(beat as i32, 0, 1, frame, writer)
            }
        })
    }

    pub fn output_notes(&self, cycle: &Cycle, channel: u8, offset: u32, interval: u32, writer: &mut Writer) -> Vec<NoteOff> {
        // Clone so we can change the tick on notes for next pattern iteration
        self.notes.iter()
            // Pattern could contain notes that fall not within start & finish of pattern
            .filter(|note| { note.tick < self.bars as u32 * self.beats_per_bar * TICKS_PER_BEAT as u32 })
            // It, is, play it, queing note off
            .filter_map(|note| {
                match cycle.delta_ticks_recurring(note.tick + offset, interval) {
                    Some(delta_ticks) => {
                        // Write note
                        writer.write(note.note_on(cycle.ticks_to_frames(delta_ticks), channel));

                        // Absolute tick note_off should be tiggered
                        Some(NoteOff::new(*note, cycle.absolute_start + delta_ticks + note.length))
                    },
                    None => None,
                }
            })
            .collect()
    }
}

