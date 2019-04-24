
use std::sync::mpsc::Sender;

use super::scroller::Scroller;
use super::{Message, RawMessage};

#[derive(Debug)]
pub struct Controller {
    pub device_id: u8,
    midi_sender: Sender<Message>,
}

impl Controller {
    pub fn new(device_id: u8, midi_sender: Sender<Message>) -> Self {
        Controller {
            device_id: device_id,
            midi_sender: midi_sender,
        }
    }

    pub fn introduce(&mut self) {
        self.midi_sender.send(Message::new(
            0,
            RawMessage::Introduction([0xF0, 0x47, self.device_id, 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]),
        ));
    }

    pub fn key_pressed(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        match event.bytes[1] {
            91 => jack_client.transport_start(),
            92 => {
                 let (state, _) = jack_client.transport_query();
                 match state {
                    1 => jack_client.transport_stop(),
                    _ => {
                        let pos = jack::Position::default();
                        jack_client.transport_reposition(pos);
                    }
                 }
            },
            _ => return,
        };
    }

    pub fn key_released(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
    
    }

    pub fn process_message(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        println!("0x{:X}, 0x{:X}, 0x{:X}", event.bytes[0], event.bytes[1], event.bytes[2]);
        println!("{}, {}, {}", event.bytes[0], event.bytes[1], event.bytes[2]);

        match event.bytes[0] {
            144 => self.key_pressed(event, jack_client),
            128 => self.key_released(event, jack_client),
            _ => {
                println!("Unknown event: {:?}", event);
            }
        }
    }

    pub fn update(&mut self) {
    }
}

