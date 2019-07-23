
pub trait Drawable {
    // led states for head & tail
    const HEAD: u8;
    const TAIL: u8;
    const MINIMUM_LENGTH: u32;

    fn visible_ticks(&self) -> u32 {
        self.length() / self.zoom()
    }

    fn ticks_per_led(&self) -> u32 {
        self.visible_ticks() / 8
    }

    fn ticks_offset(&self) -> u32 {
        self.offset() * self.visible_ticks()
    }

    fn zoom(&self) -> u32;
    fn set_zoom(&mut self, zoom: u32);
    fn offset(&self) -> u32;
    fn set_offset(&mut self, offset: u32);
    fn length(&self) -> u32;
    fn set_length(&mut self, length: u32);

    fn change_length(&mut self, length_modifier: u8) {
        match length_modifier {
            1 | 2 | 4 | 8  => {
                self.set_length(length_modifier as u32 * Drawable::MINIMUM_LENGTH);
            },
            _ => (),
        }
    }

    fn change_zoom(&mut self, button: u32) {
        let delta = match button {
            1 | 2 | 4 | 8 => Some((8 / button, 0)),
            5 => Some((2, 1)),
            7 => Some((4, 3)),
            3 | 6 => Some((8, button - 1)),
            _ => None
        };

        if let Some((zoom, offset)) = delta {
            self.set_zoom(zoom);
            self.set_offset(offset);
        }
    }

    fn change_offset(&mut self, delta: i32) {
        let offset = self.offset() as i32 + delta;

        if offset >= 0 && offset <= self.zoom() as i32 - 1 {
            self.set_offset(offset as u32);
        }
    }
    
    // Takes start-tick, end-tick, y
    fn led_states(&self, coords: Vec<(u32, u32, i32)>) -> Vec<(i32, i32, u8)> {
        return coords.into_iter()
            .flat_map(|(start, end, y)| {
                let start_led = (start as i32 - self.ticks_offset() as i32) / self.ticks_per_led() as i32;
                let total_leds = (end - start) / self.ticks_per_led();

                let mut head = vec![(start_led, y, Drawable::HEAD)];
                let tail: Vec<(i32, i32, u8)> = (1..total_leds).map(|led| (start_led + led as i32, y, Drawable::TAIL)).collect();
                head.extend(tail);
                head
            })
            .collect()
    }
}
