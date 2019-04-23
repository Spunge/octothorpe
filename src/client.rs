
use std::sync::mpsc::Sender;
use super::Message;
use super::controller::Controller;

pub struct Client {
    midi_sender: Sender<Message>, 
    bpm_sender: Sender<f64>,

    controllers: Vec<Controller>,
}

impl Client {
    pub fn new(midi_sender: Sender<Message>, bpm_sender: Sender<f64>) -> Self {
        Client {
            midi_sender: midi_sender,
            bpm_sender: bpm_sender,

            controllers: Vec::new(),
        }
    }
   
    pub fn process_midi_event(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        println!("{:?}", event);
    }
}

