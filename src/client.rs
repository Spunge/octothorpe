
use jack_sys as j;

use super::controller::Controller;

#[derive(Debug)]
pub struct Transport {
    pub is_up_to_date: bool,
    pub beats_per_minute: f64,
    pub beats_per_bar: isize,
    pub beat_type: isize,
}

pub struct Client {
    pub transport: Transport,
    pub controllers: Vec<Controller>,

    midi_out: jack::Port<jack::MidiOut>,
    midi_in: jack::Port<jack::MidiIn>,
}

impl Client {
    pub fn new(client: &jack::Client) -> Self {
        let transport = Transport {
            is_up_to_date: false,
            beats_per_bar: 4,
            beat_type: 4,
            beats_per_minute: 120.0,
        };

        // Create ports
        let midi_in = client
            .register_port("control_in", jack::MidiIn::default())
            .unwrap();
        let midi_out = client
            .register_port("control_out", jack::MidiOut::default())
            .unwrap();

        Client {
            transport: transport,
            controllers: Vec::new(),

            midi_in: midi_in,
            midi_out: midi_out,
        }
    }
   
    pub fn process_midi_event(&self, event: jack::RawMidi) {
        println!("{:?}", event);
    }
}

impl jack::ProcessHandler for Client {
    fn process(&mut self, _: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Process incoming midi
        for event in self.midi_in.iter(process_scope) {
            println!("{:?}", event);

            self.process_midi_event(event);
        }

        // process outgoing midi
        //let mut writer = self.midi_out.writer(process_scope);

        // Get buffer, output events, clear buffer
        /*
        for message in self.controller.get_midi_output() {
            match message.bytes {
                RawMessage::Introduction(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Inquiry(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
                RawMessage::Note(bytes) => 
                    writer.write(&jack::RawMidi{ time: message.time, bytes: &bytes}).unwrap(),
            }
        }
        */

        // Clear buffer after writing events
        //self.controller.clear_buffer();

        jack::Control::Continue
    }
}

impl jack::TimebaseHandler for Client {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, _n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
        unsafe {
            // Set position type
            (*pos).valid = j::JackPositionBBT;

            // Only update timebase when we are asked for it, or when our state changed
            if is_new_pos || ! self.transport.is_up_to_date {
                (*pos).beats_per_bar = self.transport.beats_per_bar as f32;
                (*pos).ticks_per_beat = 1920.0;
                (*pos).beat_type = self.transport.beat_type as f32;
                (*pos).beats_per_minute = self.transport.beats_per_minute;
                
                self.transport.is_up_to_date = true;
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

