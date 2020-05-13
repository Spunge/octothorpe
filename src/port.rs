
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
    
    pub fn write_message(&mut self, scope: &jack::ProcessScope, message: TimedMessage) {
        let mut writer = self.port.writer(scope);

        match writer.write(&message.to_raw_midi()) {
            Err(e) => {
                println!("Error: {}", e);
                println!("{:?}\n", message);
            },
            Ok(_) => (),
        }
    }

    /*
     * Sort & output messages to jack
     */
    pub fn write_messages(&mut self, scope: &jack::ProcessScope, messages: &mut Vec<TimedMessage>) {
        // Sort messages based on time in timed message as jack will complain about unordered
        // messages
        messages.sort();
        messages.drain(0..).for_each(|message| { 
            self.write_message(scope, message);
        });
    }
}
