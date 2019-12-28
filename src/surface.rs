
use super::controller::Controller;
use super::controller::input::*;

#[derive(PartialEq)]
pub enum View {
    Instrument,
    Sequence,
}

pub struct Surface {
    pub view: View,
    pub memory: Memory,

    instrument_shown: u8,
    sequence_shown: u8,
}

impl Surface {
    pub fn new() -> Self {
        Surface { 
            view: View::Instrument, 
            memory: Memory::new(),

            instrument_shown: 0,
            sequence_shown: 0,
        }
    }

    pub fn switch_view(&mut self) { 
        self.view = match self.view {
            View::Instrument => View::Sequence,
            // TODO When switching from sequence to instrument, don't note_off the instrument grid
            // Clear as we do not want the selected instrument grid to clear
            //self.indicator_note_offs = vec![];
            View::Sequence => View::Instrument,
        }
    }

    pub fn show_instrument(&mut self, index: u8) { self.instrument_shown = index; }
    pub fn instrument_shown(&self) -> usize { self.instrument_shown as usize }

    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }

    pub fn toggle_instrument(&mut self, index: u8) {
        // If we click selected instrument, return to sequence for peeking
        if self.instrument_shown() == index as usize {
            self.switch_view();
        } else {
            // Otherwise select instrument && switch
            self.show_instrument(index);
            // TODO - What does instrument target? Move this to "instrument"
            //self.keyboard_target = instrument;
            //self.drumpad_target = instrument;

            if let View::Sequence = self.view { self.switch_view() }
        }
    }

    pub fn toggle_sequence(&mut self, index: u8) {
        // When we press currently selected overview, return to instrument view, so we can peek
        if self.sequence_shown() == index as usize {
            self.switch_view();
        } else {
            // If we select a new sequence, show that
            self.show_sequence(index);

            if let View::Instrument = self.view { self.switch_view() }
        }
    }
}

#[derive(Debug)]
struct ButtonPress {
    controller_id: u8,
    start: u64,
    end: Option<u64>,
    button_type: ButtonType,
}

impl ButtonPress {
    pub fn new(controller_id: u8, start: u64, button_type: ButtonType) -> Self {
        Self { controller_id, start, end: None, button_type, }
    }
}

pub struct Memory {
    presses: Vec<ButtonPress>,
}

/*
 * This will keep track of button presses so we can support double press & range press
 */
impl Memory {
    const DOUBLE_PRESS_USECS: u64 = 300000;

    pub fn new() -> Self {
        Self { presses: vec![] }
    }

    // We pressed a button!
    pub fn press(&mut self, controller_id: u8, start: u64, button_type: ButtonType) -> bool {
        // Remove all keypresses that are not within double press range, while checking if this
        // key is double pressed wihtin short perioud
        let mut is_double_pressed = false;

        self.presses.retain(|previous| {
            let falls_within_double_press_ticks = 
                previous.end.is_none() || start - previous.end.unwrap() < Memory::DOUBLE_PRESS_USECS;

            let is_same_button = previous.button_type == button_type && previous.controller_id == controller_id;

            // Ugly side effects, but i thought this to be cleaner as 2 iters looking for the same
            // thing
            is_double_pressed = falls_within_double_press_ticks && is_same_button;

            falls_within_double_press_ticks
        });

        // Save pressed_button to compare next pressed keys with, do this after comparing to not
        // compare with current press
        self.presses.push(ButtonPress::new(controller_id, start, button_type));

        is_double_pressed
    }

    pub fn release(&mut self, controller_id: u8, end: u64, button_type: ButtonType) {
        let mut pressed_button = self.presses.iter_mut().rev()
            .find(|pressed_button| {
                pressed_button.button_type == button_type
                    && pressed_button.controller_id == controller_id
            })
            // We can safely unwrap as you can't press the same button twice
            .unwrap();

        pressed_button.end = Some(end);
    }

    pub fn modifier(&self, controller_id: u8, button_type: ButtonType) -> Option<ButtonType> {
        self.presses.iter()
            .filter(|pressed_button| {
                pressed_button.button_type != button_type
                    && pressed_button.controller_id == controller_id
            })
            .find(|pressed_button| pressed_button.end.is_none())
            .and_then(|pressed_button| Some(pressed_button.button_type))
    }

    pub fn global_modifier(&self, button_type: ButtonType) -> Option<ButtonType> {
        self.presses.iter()
            .filter(|pressed_button| pressed_button.button_type != button_type)
            .find(|pressed_button| pressed_button.end.is_none())
            .and_then(|pressed_button| Some(pressed_button.button_type))
    }
}
