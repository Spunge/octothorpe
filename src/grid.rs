
use super::message::Message;

#[derive(Debug)]
pub struct Grid {
    pub width: u8,
    pub height: u8,
    pub base_note: u8,
    active_leds: Vec<u8>,
}

// TODO - undraw & redraw?
impl Grid {
    pub fn new(width: u8, height: u8, base_note: u8) -> Self {
        Grid { width, height, base_note, active_leds: vec![] }
    }

    fn draw_led(channel: u8, note: u8, state: u8) -> Message {
        Message::Note([channel, note, state])
    }

    fn save_led_state(&mut self, led: u8, state: u8) {
        if state > 0 {
            if ! self.active_leds.contains(&led) {
                self.active_leds.push(led);
            }
        } else {
            self.active_leds.retain(|active_led| {
                &led != active_led
            })
        }
    }

    // Do not allow switching leds outside of grid
    pub fn try_switch_led(&mut self, x: i32, y: i32, state: u8) -> Option<Message> {
        if x >= self.width as i32 || x < 0 || y >= self.height as i32 || y < 0 {
            None
        } else {
            Some(self.switch_led(x as u8, y as u8, state))
        }
    }

    pub fn switch_led(&mut self, x: u8, y: u8, state: u8) -> Message {
        self.save_led_state(y * self.width + x, state);
        Grid::draw_led(0x90 + x, self.base_note + y, state)
    }

    fn clear_active(&mut self) -> Vec<Message> {
        let messages = self.active_leds.iter()
            .map(|led| { 
                Grid::draw_led(0x90 + led % self.width, self.base_note + led / self.width, 0) 
            })
            .collect();

        self.active_leds.clear();
        messages
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        if ! force {
            self.clear_active()
        } else {
            let mut messages = vec![];

            for y in 0..self.height {
                for x in 0..self.width {
                    messages.push(self.switch_led(x, y, 0));
                }
            }

            messages
        }
    }
}
