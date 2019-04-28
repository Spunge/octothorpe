
use super::message::{Message, MessageData};
use super::handlers::Writer;

pub struct Grid {
    pub width: u8,
    pub height: u8,
    base_note: u8,
    active_leds: Vec<u8>,
}

// TODO - undraw & redraw?
impl Grid {
    pub fn new(width: u8, height: u8, base_note: u8) -> Self {
        Grid { width, height, base_note, active_leds: vec![] }
    }

    fn draw_led(channel: u8, note: u8, state: u8, frame: u32, writer: &mut Writer) {
        writer.write(Message::new(frame, MessageData::Note([channel, note, state])));
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
    pub fn try_switch_led(&mut self, x: i32, y: i32, state: u8, frame: u32, writer: &mut Writer) {
        if x >= self.width as i32 || x < 0 || y >= self.height as i32 || y < 0 {
            ()
        } else {
            self.switch_led(x as u8, y as u8, state, frame, writer);
        }
    }

    pub fn switch_led(&mut self, x: u8, y: u8, state: u8, frame: u32, writer: &mut Writer) {
        self.save_led_state(y * self.width + x, state);

        Grid::draw_led(0x90 + x, self.base_note + y, state, frame, writer);
    }

    fn clear_active(&mut self, frame: u32, writer: &mut Writer) {
        self.active_leds.iter()
            .for_each(|led| {
                Grid::draw_led(0x90 + led % self.width, self.base_note + led / self.width, 0, frame, writer);
            });

        self.active_leds.clear();
    }

    pub fn clear(&mut self, frame: u32, force: bool, writer: &mut Writer) {
        if ! force {
            self.clear_active(frame, writer)
        } else {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.switch_led(x, y, 0, frame, writer);
                }
            }
        }
    }
}
