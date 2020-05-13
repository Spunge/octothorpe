
use super::controller::input::*;

#[derive(Debug)]
pub struct OccurredInputEvent {
    time: u64,
    event_type: InputEventType,
}

pub struct EventMemory {
    // Remember when the last occurence of input event was for each input event on the controller,
    // this was we can keep track of double clicks or show info based on touched buttons
    occurred_events: Vec<OccurredInputEvent>,
}

impl EventMemory {
    pub fn new() -> Self {
        Self { occurred_events: vec![] }
    }

    pub fn register_event(&mut self, time: u64, event_type: InputEventType) {
        let previous = self.occurred_events.iter_mut()
            .find(|event| event.event_type == event_type);

        if let Some(event) = previous {
            event.time = time;
        } else {
            self.occurred_events.push(OccurredInputEvent { time, event_type });
        }
    }

    pub fn last_occurred_event_after<F>(&self, filters: &[F], usecs: u64) -> Option<u64> where F: Fn(&InputEventType) -> bool {
        self.occurred_events.iter()
            .filter(|event| {
                event.time >= usecs && filters.iter().fold(false, |acc, filter| acc || filter(&event.event_type)) 
            })
            .map(|event| event.time)
            .max()
    }
}

pub struct ButtonMemory {
    // Remember pressed buttons to provide "modifier" functionality, we *could* use occurred_events
    // for this, but the logic will be a lot easier to understand when we use seperate struct
    pressed_buttons: Vec<ButtonType>,
}

/*
 * This will keep track of button presses so we can support double press & range press
 */
impl ButtonMemory {
    pub fn new() -> Self {
        Self { pressed_buttons: vec![] }
    }

    //pub fn register_event(&mut self, controller_track_offset: u8, time: u64, InputEvent:)

    // We pressed a button!
    pub fn press(&mut self, button_type: ButtonType) {
        // Save pressed_button to keep track of modifing keys (multiple keys pressed twice)
        self.pressed_buttons.push(button_type);
    }

    pub fn release(&mut self, button_type: ButtonType) {
        let pressed_button = self.pressed_buttons.iter().enumerate().rev().find(|(_, pressed_button_type)| {
            **pressed_button_type == button_type
        });

        // We only use if let instead of unwrap to not crash when first event is button release
        if let Some((index, _)) = pressed_button {
            self.pressed_buttons.remove(index);
        }
    }

    pub fn modifier(&self, button_type: ButtonType) -> Option<&ButtonType> {
        self.pressed_buttons.iter()
            .filter(|pressed_button_type| {
                **pressed_button_type != button_type
            })
            .next()
    }
}

pub struct Memory {
    pub buttons: ButtonMemory,
    pub events: EventMemory,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            buttons: ButtonMemory::new(),
            events: EventMemory::new(),
        }
    }
}
