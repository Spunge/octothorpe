
#[derive(Clone)]
pub struct Playable {
    pub minimum_length: u32,
    pub length: u32,

    pub zoom: u32,
    pub offset: u32,

    // led states for head & tail
    head: u8,
    tail: u8,
}

impl Playable {
    pub fn new(length: u32, minimum_length: u32, head: u8, tail: u8) -> Self {
        Playable { minimum_length, length, zoom: 1, offset: 0, head, tail }
    }

    pub fn visible_ticks(&self) -> u32 {
        self.length / self.zoom
    }

    pub fn ticks_per_led(&self) -> u32 {
        self.visible_ticks() / 8
    }

    pub fn ticks_offset(&self) -> u32 {
        self.offset * self.visible_ticks()
    }

    pub fn length_modifier(&self) -> u32 {
        self.length / self.minimum_length 
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
    
    pub fn change_length(&mut self, length_modifier: u32) -> Option<u32> {
        match length_modifier {
            1 | 2 | 4 | 8  => {
                self.length = length_modifier * self.minimum_length;
                // Check if offset is still okay
                if self.offset > self.zoom - 1 {
                    self.offset = self.zoom - 1;
                }

                Some(length_modifier)
            },
            _ => None,
        }
    }

}
