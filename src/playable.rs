
use super::{TICKS_PER_BEAT, BEATS_PER_BAR};
use super::message::Message;
use super::grid::Grid;

pub struct Playable {
    minimum_ticks: u32,
    pub ticks: u32,
    pub zoom: u32,
    pub offset: u32,

    pub main_grid: Grid,
    pub length_grid: Grid,
    pub zoom_grid: Grid,
}

fn bars_to_ticks(bars: u8) -> u32 {
    bars as u32 * BEATS_PER_BAR as u32 * TICKS_PER_BEAT as u32
}

impl Playable {
    pub fn new(bars: u8, minimum_bars: u8) -> Self {
        Playable {
            minimum_ticks: bars_to_ticks(minimum_bars),
            ticks: bars_to_ticks(bars),
            zoom: 1, 
            offset: 0,

            main_grid: Grid::new(8, 5, 0x35),
            length_grid: Grid::new(8, 1, 0x32),
            zoom_grid: Grid::new(8, 1, 0x31),
        }
    }

    pub fn ticks_per_led(&self) -> u32 {
        self.ticks / self.zoom / self.main_grid.width as u32
    }

    pub fn ticks_offset(&self) -> u32 {
        self.main_grid.width as u32 * self.offset * self.ticks_per_led()
    }

    pub fn beats(&self) -> u32 {
        self.ticks / TICKS_PER_BEAT as u32
    }

    pub fn bars(&self) -> u32 {
        self.beats() / BEATS_PER_BAR as u32
    }

    pub fn coords_to_leds(&self, coords: Vec<(u32, u32, i32)>) -> Vec<(i32, i32, u8)> {
        return coords.into_iter()
            .flat_map(|(start, end, y)| {
                let start_led = (start as i32 - self.ticks_offset() as i32) / self.ticks_per_led() as i32;
                let total_leds = (end - start) / self.ticks_per_led();

                let mut head = vec![(start_led, y, 1)];
                let tail: Vec<(i32, i32, u8)> = (1..total_leds).map(|led| (start_led + led as i32, y, 5)).collect();
                head.extend(tail);
                head
            })
            .collect()
    }

    pub fn try_switch_coords(&mut self, coords: Vec<(u32, u32, i32)>) -> Vec<Message> {
        self.coords_to_leds(coords).into_iter()
            .filter_map(|(x, y, state)| { self.main_grid.try_switch_led(x, y, state) }).collect()
    }

    fn length_modifier(&self) -> u32 {
        self.ticks / self.minimum_ticks
    }

    pub fn change_zoom(&mut self, button: u32) -> bool {
        match button {
            1 | 2 | 4 | 8 => { self.zoom = 8 / button; self.offset = 0; true },
            5 => { self.zoom = 2; self.offset = 1; true },
            7 => { self.zoom = 4; self.offset = 3; true },
            3 | 6 => { self.zoom = 8; self.offset = button - 1; true },
            _ => false
        }
    }

    pub fn change_offset(&mut self, delta: i32) -> bool {
        let offset = self.offset as i32 + delta;

        if offset >= 0 && offset <= self.zoom as i32 - 1 {
            self.offset = offset as u32;
            true
        } else {
            false
        }
    }
    
    pub fn change_length(&mut self, length_modifier: u8) -> bool {
        match length_modifier {
            1 | 2 | 4 | 8  => {
                // Calculate new zoom level to keep pattern grid view the same if possible
                let zoom = self.zoom * length_modifier as u32 / self.length_modifier() as u32;
                self.ticks = length_modifier as u32 * self.minimum_ticks;
                // Only set zoom when it's possible
                if zoom > 0 && zoom <= 8 {
                    self.zoom = zoom;
                }
                true
            },
            _ => false,
        }
    }

    pub fn draw_length(&mut self) -> Vec<Message> {
        (0..self.length_modifier()).map(|x| { self.length_grid.switch_led(x as u8, 0, 1) }).collect()
    }

    pub fn draw_zoom(&mut self) -> Vec<Message> {
        let length = 8 / self.zoom;
        let from = self.offset * length;
        let to = from + length;

        (from..to)
            .map(|x| { self.zoom_grid.switch_led(x as u8, 0, 1) })
            .collect()
    }

}
