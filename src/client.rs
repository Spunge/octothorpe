
use super::transport::Transport;
use super::controller::Controller;
//use super::RawMessage;

pub struct Client {
    pub transport: Transport,
    pub controllers: Vec<Controller>,
}

impl Client {
    pub fn new() -> Self {
        let transport = Transport {
            beats_per_bar: 4,
            beat_type: 4,
            beats_per_minute: 120.0,
        };

        Client {
            transport: transport,
            controllers: Vec::new(),
        }
    }
   
    pub fn process_midi_event(&self, event: jack::RawMidi) {

    }
}

