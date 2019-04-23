
use jack_sys as j;

use super::client::Client;
use super::Message;

pub struct ProcessHandler<'a> {
    client: &'a Client,

    buffer: Vec<Message>,

    midi_out: jack::Port<jack::MidiOut>,
    midi_in: jack::Port<jack::MidiIn>,
}

impl<'a> ProcessHandler<'a> {
    pub fn new(jack_client: &jack::Client, client: &'a mut Client) -> Self {
        // Create ports
        let midi_in = jack_client
            .register_port("control_in", jack::MidiIn::default())
            .unwrap();
        let midi_out = jack_client
            .register_port("control_out", jack::MidiOut::default())
            .unwrap();

        ProcessHandler {
            client: client,

            buffer: Vec::new(),

            midi_in: midi_in,
            midi_out: midi_out,
        }
    }
}

impl<'a> jack::ProcessHandler for ProcessHandler<'a> {
    fn process(&mut self, _: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Process incoming midi
        for event in self.midi_in.iter(process_scope) {
            println!("{:?}", event);

            self.client.process_midi_event(event);
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

pub struct TimebaseHandler<'a> {
    is_up_to_date: bool,
    client: &'a Client,
}

impl<'a> TimebaseHandler<'a> {
    pub fn new(client: &'a Client) -> Self {
        TimebaseHandler {
            is_up_to_date: false,
            client: client,
        }
    }
}

impl<'a> jack::TimebaseHandler for TimebaseHandler<'a> {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, _n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
        unsafe {
            // Set position type
            (*pos).valid = j::JackPositionBBT;

            // Only update timebase when we are asked for it, or when our state changed
            if is_new_pos || ! self.is_up_to_date {
                (*pos).beats_per_bar = self.client.transport.beats_per_bar as f32;
                (*pos).ticks_per_beat = 1920.0;
                (*pos).beat_type = self.client.transport.beat_type as f32;
                (*pos).beats_per_minute = self.client.transport.beats_per_minute;
                
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
