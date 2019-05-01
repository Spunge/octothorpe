
use super::message::Message;
use super::grid::Grid;

pub struct Playable {
    minimum_beats: u8,
    pub beats: u8,
    pub zoom: u32,
    pub offset: u32,

    pub pattern_grid: Grid,
    pub length_grid: Grid,
    pub zoom_grid: Grid,
}

impl Playable {
    pub fn new(beats: u8, minimum_beats: u8) -> Self {
        Playable {
            minimum_beats,
            beats,
            zoom: 1, 
            offset: 0,

            pattern_grid: Grid::new(8, 5, 0x35),
            length_grid: Grid::new(8, 1, 0x32),
            zoom_grid: Grid::new(8, 1, 0x31),
        }
    }

    fn length_modifier(&self) -> u8 {
        self.beats / self.minimum_beats
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
    
    pub fn change_length(&mut self, length_modifier: u8) {
        match length_modifier {
            1 | 2 | 4 | 8  => {
                // Calculate new zoom level to keep pattern grid view the same if possible
                let zoom = self.zoom * length_modifier as u32 / self.length_modifier() as u32;
                self.beats = length_modifier * self.minimum_beats;
                // Only set zoom when it's possible
                if zoom > 0 && zoom <= 8 {
                    self.zoom = zoom;
                }
            },
            _ => {},
        }
    }

    pub fn draw_length(&mut self) -> Vec<Message> {
        (0..self.length_modifier()).map(|x| { self.length_grid.switch_led(x, 0, 1) }).collect()
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
