
use super::message::{TimedMessage, Message};

pub struct MidiOut {
    pub messages: Vec<TimedMessage>,

    pub port: jack::Port<jack::MidiOut>,
}

// We use a wrapper so we can sort the messages before outputting them to jack, as out off order
// messages produce runtime errors
impl MidiOut {
    pub fn new(port: jack::Port<jack::MidiOut>) -> Self {
        MidiOut { port, messages: vec![] }
    }

    /*
     * Write messages to buffer
     */
    pub fn output_messages(&mut self, messages: &mut Vec<TimedMessage>) {
        self.messages.append(messages);
    }

    pub fn output_message(&mut self, message: TimedMessage) {
        //dbg!(&message);
        self.messages.push(message);
    }

    pub fn clear_output_buffer(&mut self) {
        self.messages = vec![];
    }

    /*
     * Output to jack
     */
    pub fn write_midi(&mut self, process_scope: &jack::ProcessScope) {
        let mut writer = self.port.writer(process_scope);

        // Sort messages based on time in timed message as jack will complain about unordered
        // messages
        self.messages.sort();
        self.messages.drain(0..).for_each(|message| { 
            match writer.write(&message.to_raw_midi()) {
                Err(e) => {
                    println!("Error: {}", e);
                    println!("{:?}\n", message);
                },
                Ok(_) => {},
            }
        });
    }
}
