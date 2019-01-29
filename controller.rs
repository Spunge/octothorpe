
pub struct Controller {
}

impl Controller {
    // Create new controller
    pub fn new() -> Controller {
        Controller {}
    }

    // Write midi events to output
    pub fn write_midi_events(&self, _writer: jack::MidiWriter) {
        println!("One of the ports got connected, sending identify request");
    }

    // Try to identify connected controller
    pub fn identify(&self) {
        println!("One of the ports got connected, sending identify request");
    }
}
