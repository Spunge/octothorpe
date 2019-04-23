

use super::scroller::Scroller;
use super::{Message, RawMessage};

#[derive(Debug)]
pub struct Controller {
    is_identified: bool,
    device_id: u8,

    scroller: Scroller,

    tick_counter: usize,
    ticks_per_frame: usize,
}

impl Controller {
    pub fn new() -> Self {
        Controller {
            is_identified: false,
            device_id: 0,

            tick_counter: 0,
            ticks_per_frame: 30,

            scroller: Scroller::new("testing".to_string()),
        }
    }

    fn inquire(&mut self, buffer: &mut Vec<Message>) {
        buffer.push(Message::new(
            0, 
            RawMessage::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]),
        ));
    }

    fn identify(&self, inquiry_response: jack::RawMidi, buffer: &mut Vec<Message>) {
        // 0x47 = akai manufacturer, 0x73 = model nr
        if inquiry_response.bytes[5] == 0x47 && inquiry_response.bytes[6] == 0x73  {
            println!("Identified APC40");

            //self.is_identified = true;
            //self.device_id = inquiry_response.bytes[13];

            buffer.push(Message::new(
                0,
                RawMessage::Introduction([0xF0, 0x47, self.device_id, 0x73, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]),
            ));
        }
    }

    fn process_sysex_message(&self, event: jack::RawMidi, _buffer: &mut Vec<Message>) {
        // 0x06 = inquiry message, 0x02 = inquiry response
        if event.bytes[3] == 0x06 && event.bytes[4] == 0x02  {
            println!("Got inquiry response!");
            //self.identify(event, buffer);
        } else {
            println!("Got Sysex!");
        }
    }

    fn process_message(&self, event: jack::RawMidi, _buffer: &mut Vec<Message>) {
        println!("Got Midi!");
        println!("{:?}", event);
    }

    pub fn process_midi_event(&self, event: jack::RawMidi, buffer: &mut Vec<Message>) {
        // Sysex events pass us a lot of data
        // It's cleaner to check the first byte though
        if event.bytes.len() > 3 {
            //self.process_sysex_message(event, buffer)
        } else {
            self.process_message(event, buffer);
        }
    }

    pub fn output_midi(&mut self, buffer: &mut Vec<Message>) {
        self.tick_counter += 1;

        if ! self.is_identified {
            self.inquire(buffer);
        } else {
            //self.print_frame();
        }
    }

    /*
    fn print_frame(&mut self) {
        // Is it time to draw?
        if self.tick_counter % self.ticks_per_frame == 0 {
            let mut frame = self.scroller.get_frame();
            self.scroller.next_frame();

            self.buffer.append(&mut frame);
        }
    }
    */
}

