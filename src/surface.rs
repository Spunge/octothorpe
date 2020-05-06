
use super::controller::input::*;

#[derive(Debug, PartialEq)]
pub enum View {
    Track,
    Sequence,
    Timeline,
}

pub struct Surface {
    pub view: View,
    pub button_memory: ButtonMemory,
    pub event_memory: EventMemory,

    track_shown: u8,
    sequence_shown: u8,
    pub timeline_offset: u32,
}

impl Surface {
    pub fn new() -> Self {
        Surface { 
            view: View::Track, 
            button_memory: ButtonMemory::new(),
            event_memory: EventMemory::new(),

            track_shown: 0,
            sequence_shown: 0,
            timeline_offset: 0,
        }
    }

    pub fn switch_view(&mut self, view: View) { 
        self.view = view;
    }

    pub fn show_track(&mut self, index: u8) { self.track_shown = index; }
    pub fn track_shown(&self) -> usize { self.track_shown as usize }

    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }
}

#[derive(Debug)]
struct OccurredInputEvent {
    controller_id: u8,
    time: u64,
    event_type: InputEventType,
}

pub struct EventMemory {
    // Remember when the last occurence of input event was for each input event on the controller,
    // this was we can keep track of double clicks or show info based on touched buttons
    occurred_events: Vec<OccurredInputEvent>,
}

impl EventMemory {
    fn new() -> Self {
        Self { occurred_events: vec![] }
    }

    pub fn register_event(&mut self, controller_id: u8, time: u64, event_type: InputEventType) {
        let previous = self.occurred_events.iter_mut()
            .find(|event| event.controller_id == controller_id && event.event_type == event_type);

        if let Some(event) = previous {
            event.time = time;
        } else {
            self.occurred_events.push(OccurredInputEvent { controller_id, time, event_type });
        }
    }

    pub fn last_occurred_event_after<F>(&self, controller_id: u8, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
        self.occurred_events.iter()
            .filter(|event| {
                controller_id == event.controller_id
                    && event.time >= usecs
                    && filters.iter().fold(false, |acc, filter| acc || filter(&event.event_type)) 
            })
            .map(|event| event.time)
            .max()
    }
}

#[derive(Debug)]
struct ButtonPress {
    controller_id: u8,
    button_type: ButtonType,
}

pub struct ButtonMemory {
    // Remember pressed buttons to provide "modifier" functionality, we *could* use occurred_events
    // for this, but the logic will be a lot easier to understand when we use seperate struct
    pressed_buttons: Vec<ButtonPress>,
}

/*
 * This will keep track of button presses so we can support double press & range press
 */
impl ButtonMemory {
    pub fn new() -> Self {
        Self { pressed_buttons: vec![] }
    }

    //pub fn register_event(&mut self, controller_id: u8, time: u64, InputEvent:)

    // We pressed a button!
    pub fn press(&mut self, controller_id: u8, button_type: ButtonType) {
        // Save pressed_button to keep track of modifing keys (multiple keys pressed twice)
        self.pressed_buttons.push(ButtonPress { controller_id, button_type, });
    }

    pub fn release(&mut self, controller_id: u8, _end: u64, button_type: ButtonType) {
        let pressed_button = self.pressed_buttons.iter().enumerate().rev().find(|(_, pressed_button)| {
            pressed_button.button_type == button_type
                && pressed_button.controller_id == controller_id
        });

        // We only use if let instead of unwrap to not crash when first event is button release
        if let Some((index, _)) = pressed_button {
            self.pressed_buttons.remove(index);
        }
    }

    pub fn modifier(&self, controller_id: u8, button_type: ButtonType) -> Option<ButtonType> {
        self.pressed_buttons.iter()
            .filter(|pressed_button| {
                pressed_button.button_type != button_type
                    && pressed_button.controller_id == controller_id
            })
            .next()
            .and_then(|pressed_button| Some(pressed_button.button_type))
    }

    pub fn global_modifier(&self, button_type: ButtonType) -> Option<ButtonType> {
        self.pressed_buttons.iter()
            .filter(|pressed_button| pressed_button.button_type != button_type)
            .next()
            .and_then(|pressed_button| Some(pressed_button.button_type))
    }
}
