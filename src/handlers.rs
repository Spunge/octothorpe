
use jack_sys as j;

use std::sync::mpsc::{Sender, Receiver};
use super::client::Client;
use super::{Message, RawMessage};

pub struct TimebaseHandler {
    receiver: Receiver<f64>,

    beats_per_minute: f64,
    beats_per_bar: isize,
    beat_type: isize,
    is_up_to_date: bool,
}

impl TimebaseHandler {
    pub fn new(receiver: Receiver<f64>) -> Self {
        TimebaseHandler {
            receiver: receiver,

            beats_per_minute: 120.0,
            is_up_to_date: false,
            beats_per_bar: 4,
            beat_type: 4,
        }
    }
}

impl jack::TimebaseHandler for TimebaseHandler {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, _n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
        unsafe {
            // Set position type
            (*pos).valid = j::JackPositionBBT;

            // Only update timebase when we are asked for it, or when our state changed
            if is_new_pos || ! self.is_up_to_date {
                (*pos).beats_per_bar = self.beats_per_bar as f32;
                (*pos).ticks_per_beat = 1920.0;
                (*pos).beat_type = self.beat_type as f32;
                (*pos).beats_per_minute = self.beats_per_minute;
                
                self.is_up_to_date = true;
            }

            let second = (*pos).frame as f64 / (*pos).frame_rate as f64;

            let abs_tick = second / 60.0 * (*pos).beats_per_minute * (*pos).ticks_per_beat;
            let abs_beat = abs_tick / (*pos).ticks_per_beat;

            (*pos).bar = (abs_beat / (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).beat = (abs_beat % (*pos).beats_per_bar as f64) as i32;
            (*pos).bar_start_tick = (abs_beat as i32 * (*pos).ticks_per_beat as i32) as f64;
            (*pos).tick = abs_tick as i32 - (*pos).bar_start_tick as i32;
        }
    }
}

pub struct ProcessHandler {
    client: Client,
    midi_receiver: Receiver<Message>,

    midi_out: jack::Port<jack::MidiOut>,
    midi_in: jack::Port<jack::MidiIn>,
}

impl ProcessHandler {
    pub fn new(midi_receiver: Receiver<Message>, client: Client, jack_client: &jack::Client) -> Self {
        // Create ports
        let midi_in = jack_client
            .register_port("control_in", jack::MidiIn::default())
            .unwrap();
        let midi_out = jack_client
            .register_port("control_out", jack::MidiOut::default())
            .unwrap();

        ProcessHandler {
            client: client,
            midi_receiver: midi_receiver,

            midi_in: midi_in,
            midi_out: midi_out,
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, jack_client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Process incoming midi
        for event in self.midi_in.iter(process_scope) {
            self.client.process_midi_event(event, jack_client);
        }

        self.client.update();

        // process outgoing midi
        let mut writer = self.midi_out.writer(process_scope);

        while let Ok(message) = self.midi_receiver.try_recv() {
            match message.bytes {
                RawMessage::Introduction(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Inquiry(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Note(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
            }
        }

        jack::Control::Continue
    }
}

pub struct NotificationHandler {
    midi_sender: Sender<Message>,
}

impl NotificationHandler {
    pub fn new(midi_sender: Sender<Message>) -> Self {
        NotificationHandler {
            midi_sender: midi_sender,
        }
    }
}

impl jack::NotificationHandler for NotificationHandler {
    fn ports_connected(&mut self, jack_client: &jack::Client, id_a: jack::PortId, id_b: jack::PortId, are_connected: bool) {
        let port_a = jack_client.port_by_id(id_a).unwrap();
        let port_b = jack_client.port_by_id(id_b).unwrap();

        if (jack_client.is_mine(&port_a) || jack_client.is_mine(&port_b)) && are_connected {
            println!("Sending inquiry");

            let message = Message::new(0, RawMessage::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]));

            self.midi_sender.send(message);
        }
    }
}

