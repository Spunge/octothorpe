
pub struct Mixer {
}

impl Mixer {
    pub fn new() -> Self {
        Self {}
    }

    /*
     * TODO - Output these over OSC directly to non-mixer
     */
    pub fn fader_adjusted(&mut self, time: u32, fader: u8, value: u8) {
        println!("fader {:?} adjusted to {:?}", fader, value);
        // TODO - Output this to corresponding port
        //vec![TimedMessage::new(time, Message::Note([0xB0 + 15, out_knob, value]))]
    }

    /*
     * TODO - Output these over OSC directly to non-mixer
     */
    pub fn master_adjusted(&mut self, time: u32, value: u8) {
        println!("master adjusted to {:?}", value);
        // TODO - Output this to corresponding port
        //vec![TimedMessage::new(time, Message::Note([0xB0 + 15, 127, value]))]
    }

}
