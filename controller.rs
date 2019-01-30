


pub struct Controller { 
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
        }
    }

    pub fn process_midi_input(&self, iterator: jack::MidiIter) {
        for event in iterator {
            println!("{:?}", event);
        }
    }

    // Write midi events to output
    pub fn write_midi_output(&self, _writer: jack::MidiWriter) {
        //println!("Writing my queued midi events now");
    }

    // Try to identify connected controller
    pub fn identify(&self) {
        println!("One of my ports got connected, sending identify request");
    }
}

