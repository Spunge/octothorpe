


pub struct Controller<'a> { 
    ring_buffer: &'a jack::RingBuffer,
}

impl<'a> Controller<'a> {
    pub fn new(ring_buffer: &jack::RingBuffer) -> Self {
    //let (mut reader, mut writer) = ringbuf.into_reader_writer();

        Controller { 
            output_buffer: Vec::new(),
        }
    }

    pub fn process_midi_input(&self, iterator: jack::MidiIter) {
        for event in iterator {
            println!("{:?}", event);
        }
    }

    // Write midi events to output
    pub fn write_midi_output(&self, _writer: jack::MidiWriter) {
        //for event in self.output_buffer.drain(..) {
            //println!("{:?}", event);
        //}
    }

    // Try to identify connected controller
    pub fn identify(&self) {
        println!("One of my ports got connected, sending identify request");

        /*
        let event = jack::RawMidi {
            time: 0,
            bytes: &[
                0b10010000 /* Note On, channel 1 */, 0b01000000 /* Key number */,
                0b01111111 /* Velocity */,
            ],
        };
    */

        //self.output_buffer.push(event);
    }
}

