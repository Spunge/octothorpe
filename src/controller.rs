
use std::sync::mpsc::Sender;

use super::scroller::Scroller;
use super::{Message, RawMessage};

#[derive(Debug)]
pub struct Controller {
    pub device_id: u8,
    midi_sender: Sender<Message>,

    scroller: Scroller,

    tick_counter: usize,
    ticks_per_frame: usize,
}

impl Controller {
    pub fn new(device_id: u8, midi_sender: Sender<Message>) -> Self {
        Controller {
            device_id: device_id,
            midi_sender: midi_sender,

            tick_counter: 0,
            ticks_per_frame: 30,

            scroller: Scroller::new(device_id.to_string()),
        }
    }

    pub fn introduce(&mut self) {
        self.midi_sender.send(Message::new(
            0,
            RawMessage::Introduction([0xF0, 0x47, self.device_id, 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]),
        ));
    }

    pub fn process_midi_event(&self, event: jack::RawMidi, buffer: &mut Vec<Message>) {
    }

    pub fn update(&mut self) {
        self.print_frame();

        self.tick_counter += 1;
    }

    fn print_frame(&mut self) {
        // Is it time to draw?
        if self.tick_counter % self.ticks_per_frame == 0 {
            self.scroller.print_frame(&mut self.midi_sender);
            self.scroller.next_frame();
        }
    }
}

