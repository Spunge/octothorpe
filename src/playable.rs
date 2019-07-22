
use super::handlers::TimebaseHandler;

pub struct Playable {
    pub minimum_ticks: u32,
    pub ticks: u32,

    pub zoom: u32,
    pub offset: u32,

    // led states for head & tail
    head: u8,
    tail: u8,
}

trait Playable {
    fn beats_to_ticks(beats: f64) -> u32 {
        (beats * TimebaseHandler::TICKS_PER_BEAT as f64) as u32
    }

    fn bars_to_beats(bars: u32) -> u32 {
        bars * TimebaseHandler::BEATS_PER_BAR
    }

    fn bars_to_ticks(bars: u32) -> u32 {
        bars_to_beats(bars) * TimebaseHandler::TICKS_PER_BEAT
    }

    // Get length of this playable in ticks
    pub fn length(&self) {
        self.length
    }
}

trait Drawable {
    pub fn visible_ticks(&self) -> u32 {
        self.length() / self.zoom
    }

    pub fn ticks_per_led(&self) -> u32 {
        self.visible_ticks() / 8
    }

    pub fn ticks_offset(&self) -> u32 {
        self.offset * self.visible_ticks()
    }

    pub fn change_zoom(&mut self, button: u32) {
        match button {
            1 | 2 | 4 | 8 => { self.zoom = 8 / button; self.offset = 0 },
            5 => { self.zoom = 2; self.offset = 1 },
            7 => { self.zoom = 4; self.offset = 3 },
            3 | 6 => { self.zoom = 8; self.offset = button - 1 },
            _ => ()
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
                self.length = length_modifier as u32 * self::MINIMUM_LENGTH;
            },
            _ => (),
        }
    }

    // Takes start-tick, end-tick, y
    pub fn led_states(&self, coords: Vec<(u32, u32, i32)>) -> Vec<(i32, i32, u8)> {
        return coords.into_iter()
            .flat_map(|(start, end, y)| {
                let start_led = (start as i32 - self.ticks_offset() as i32) / self.ticks_per_led() as i32;
                let total_leds = (end - start) / self.ticks_per_led();

                let mut head = vec![(start_led, y, self.head)];
                let tail: Vec<(i32, i32, u8)> = (1..total_leds).map(|led| (start_led + led as i32, y, self.tail)).collect();
                head.extend(tail);
                head
            })
            .collect()
    }
}

impl Playable {
    pub fn new(ticks: u32, minimum_ticks: u32, head: u8, tail: u8) -> Self {
        Playable {
            minimum_ticks,
            ticks,
            zoom: 1, 
            offset: 0,
            head,
            tail,
        }
    }

    pub fn visible_ticks(&self) -> u32 {
        self.ticks / self.zoom
    }

    pub fn ticks_per_led(&self) -> u32 {
        self.visible_ticks() / 8
    }

    pub fn ticks_offset(&self) -> u32 {
        self.offset * self.visible_ticks()
    }

    // Takes start-tick, end-tick, y
    pub fn led_states(&self, coords: Vec<(u32, u32, i32)>) -> Vec<(i32, i32, u8)> {
        return coords.into_iter()
            .flat_map(|(start, end, y)| {
                let start_led = (start as i32 - self.ticks_offset() as i32) / self.ticks_per_led() as i32;
                let total_leds = (end - start) / self.ticks_per_led();

                let mut head = vec![(start_led, y, self.head)];
                let tail: Vec<(i32, i32, u8)> = (1..total_leds).map(|led| (start_led + led as i32, y, self.tail)).collect();
                head.extend(tail);
                head
            })
            .collect()
    }

    pub fn change_zoom(&mut self, button: u32) {
        match button {
            1 | 2 | 4 | 8 => { self.zoom = 8 / button; self.offset = 0 },
            5 => { self.zoom = 2; self.offset = 1 },
            7 => { self.zoom = 4; self.offset = 3 },
            3 | 6 => { self.zoom = 8; self.offset = button - 1 },
            _ => ()
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
                //let zoom = self.zoom * length_modifier as u32 / (self.ticks / self.minimum_ticks);
                self.ticks = length_modifier as u32 * self.minimum_ticks;
                // Only set zoom when it's possible
                //if zoom > 0 && zoom <= 8 {
                    //self.zoom = zoom;
                //}
                // Check if offset is still okay
                if self.offset > self.zoom - 1 {
                    self.offset = self.zoom - 1;
                }
            },
            _ => (),
        }
    }

}
