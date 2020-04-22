
use super::message::TimedMessage;

pub struct MidiOut {
    pub port: jack::Port<jack::MidiOut>,
}

// We use a wrapper so we can sort the messages before outputting them to jack, as out off order
// messages produce runtime errors
impl MidiOut {
    pub fn new(port: jack::Port<jack::MidiOut>) -> Self {
        MidiOut { port }
    }

    /*
     * Output to jack
     */
    pub fn write_midi(&mut self, process_scope: &jack::ProcessScope, messages: &mut Vec<TimedMessage>) {
        let mut writer = self.port.writer(process_scope);

        // Sort messages based on time in timed message as jack will complain about unordered
        // messages
        messages.sort();
        messages.drain(0..).for_each(|message| { 
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
