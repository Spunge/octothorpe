
use std::sync::mpsc::Sender;
use super::Message;
use super::controller::Controller;

pub struct Client {
    midi_sender: Sender<Message>, 
    bpm_sender: Sender<f64>,

    controller: Option<Controller>,
}

impl Client {
    pub fn new(midi_sender: Sender<Message>, bpm_sender: Sender<f64>) -> Self {
        Client {
            midi_sender: midi_sender,
            bpm_sender: bpm_sender,

            controller: None,
        }
    }

    pub fn update(&mut self) {
        if self.controller.is_some() {
            self.controller.as_mut().unwrap().update();
        }
    }
    
    fn initialize_controller(&mut self, device_id: u8) {
        if let None = self.controller {
            let mut controller = Controller::new(device_id, self.midi_sender.clone());
            controller.introduce();

            self.controller = Some(controller);
        }
    }

    pub fn process_midi_event(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        // Sysex events pass us a lot of data
        // It's cleaner to check the first byte though
        if event.bytes.len() > 3 {
            self.process_sysex_message(event);
        } else {
            self.process_message(event, jack_client);
        }
    }

    fn process_sysex_message(&mut self, event: jack::RawMidi) {
        // 0x06 = inquiry message, 0x02 = inquiry response
        if event.bytes[3] == 0x06 && event.bytes[4] == 0x02  {
            // 0x47 = akai manufacturer, 0x73 = model nr
            if event.bytes[5] == 0x47 && event.bytes[6] == 0x73 {
                println!("{:?}", event);
                self.initialize_controller(event.bytes[13]);
            }
        } else {
            println!("Got unknown sysex message");
            println!("{:?}", event);
        }
    }

    fn process_message(&mut self, event: jack::RawMidi, jack_client: &jack::Client) {
        if self.controller.is_some() {
            self.controller.as_mut().unwrap().process_message(event, jack_client);
        }
    }
}

