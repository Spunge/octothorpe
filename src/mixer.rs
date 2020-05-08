
use super::port::*;
use super::message::*;
use super::cycle::*;

pub struct Mixer {
    output: MidiOut,
    buffer: Vec<TimedMessage>,
}

impl Mixer {
    pub fn new(client: &jack::Client) -> Self {
        let output = client.register_port("Mixer", jack::MidiOut::default()).unwrap();

        Self { 
            output: MidiOut::new(output),
            buffer: vec![],
        }
    }

    pub fn fader_adjusted(&mut self, time: u32, fader: u8, value: u8) {
        // TODO - Output this to corresponding port
        self.buffer.push(TimedMessage::new(time, Message::Note([0xB0, fader, value])));
    }

    pub fn master_adjusted(&mut self, time: u32, value: u8) {
        // TODO - Output this to corresponding port
        //vec![TimedMessage::new(time, Message::Note([0xB0 + 15, 127, value]))]
        self.buffer.push(TimedMessage::new(time, Message::Note([0xB0, 127, value])));
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        // This wil drain buffer as write_midi uses vec.drain
        self.output.write_midi(cycle.scope, &mut self.buffer);
    }
}
