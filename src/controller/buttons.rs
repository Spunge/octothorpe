
use super::super::handlers::TimebaseHandler;

#[derive(Debug)]
struct PressedButton {
    start: u32,
    end: Option<u32>,
    channel: u8,
    note: u8,
}

impl PressedButton {
    pub fn new(start: u32, channel: u8, note: u8) -> Self {
        Self { start, end: None, channel, note }
    }
}

pub struct Buttons {
    pressed: Vec<PressedButton>,
}

impl Buttons {
    const DOUBLE_PRESS_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT / 2;

    pub fn new() -> Self {
        Self { pressed: vec![] }
    }

    // We pressed a button!
    pub fn press(&mut self, start: u32, channel: u8, note: u8) -> bool {
        // Remove all keypresses that are not within double press range, while checking if this
        // key is double pressed wihtin short perioud
        let mut is_double_pressed = false;

        self.pressed.retain(|previous| {
            let falls_within_double_press_ticks = 
                previous.end.is_none() || start - previous.end.unwrap() < Buttons::DOUBLE_PRESS_TICKS;

            let is_same_button = 
                previous.channel == channel && previous.note == note;

            // Ugly side effects, but i thought this to be cleaner as 2 iters looking for the same
            // thing
            is_double_pressed = falls_within_double_press_ticks && is_same_button;

            falls_within_double_press_ticks
        });

        // Save pressed_button to compare next pressed keys with, do this after comparing to not
        // compare with current press
        self.pressed.push(PressedButton::new(start, channel, note));

        is_double_pressed
    }

    pub fn release(&mut self, end: u32, channel: u8, note: u8) {
        let mut pressed_button = self.pressed.iter_mut().rev()
            .find(|pressed_button| {
                // press = 0x90, release = 0x80
                pressed_button.channel - 16 == channel && pressed_button.note == note
            })
            // We can safely unwrap as you can't press the same button twice
            .unwrap();

        pressed_button.end = Some(end);
    }
}

