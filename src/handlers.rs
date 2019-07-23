
use jack_sys as j;

use std::sync::mpsc::{Sender, Receiver};
use super::controller::Controller;
use super::message::{TimedMessage, Message};
use super::cycle::Cycle;

pub struct TimebaseHandler {
    beats_per_minute: f64,
    beats_per_bar: isize,
    beat_type: isize,
    is_up_to_date: bool,
}

impl TimebaseHandler {
    const TICKS_PER_BEAT: u32 = 1920;
    const BEATS_PER_BAR: u32 = 4;

    pub fn beats_to_ticks(beats: f64) -> u32 {
        (beats * Self::TICKS_PER_BEAT as f64) as u32
    }

    pub fn bars_to_beats(bars: u32) -> u32 {
        bars * Self::BEATS_PER_BAR
    }

    pub fn bars_to_ticks(bars: u32) -> u32 {
        Self::bars_to_beats(bars) * Self::TICKS_PER_BEAT
    }

    pub fn new() -> Self {
        TimebaseHandler {
            beats_per_minute: 130.0,
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

            // BPM changed?
            //if ! is_new_pos && (*pos).beats_per_minute != self.beats_per_minute {
                //println!("{:?}", (*pos).beats_per_minute);
            //}

            // Only update timebase when we are asked for it, or when our state changed
            if is_new_pos || ! self.is_up_to_date {
                (*pos).beats_per_bar = self.beats_per_bar as f32;
                (*pos).ticks_per_beat = Self::TICKS_PER_BEAT as f64;
                (*pos).beat_type = self.beat_type as f32;
                (*pos).beats_per_minute = self.beats_per_minute;
                
                self.is_up_to_date = true;
            }

            let abs_tick = Cycle::get_tick(*pos, (*pos).frame);
            let abs_beat = abs_tick / (*pos).ticks_per_beat;

            (*pos).bar = (abs_beat / (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).beat = (abs_beat % (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).bar_start_tick = (abs_beat as i32 * (*pos).ticks_per_beat as i32) as f64;
            (*pos).tick = abs_tick as i32 - (*pos).bar_start_tick as i32;
        }
    }
}

struct MidiOut {
    port: jack::Port<jack::MidiOut>,
}

impl MidiOut {
    fn write(&mut self, process_scope: &jack::ProcessScope, mut messages: Vec<TimedMessage>) {
        let mut writer = self.port.writer(process_scope);

        messages.sort();
        messages.iter().for_each(|message| { 
            match writer.write(&message.to_raw_midi()) {
                Err(e) => {
                    println!("Error: {}", e);
                    println!("{:?}\n", messages);
                },
                Ok(_) => {},
            }
        });
    }
}

pub struct ProcessHandler {
    controller: Controller,
    receiver: Receiver<TimedMessage>,

    ticks_elapsed: u32,
    was_repositioned: bool,

    control_in: jack::Port<jack::MidiIn>,
    control_out: MidiOut,

    midi_out: MidiOut,
}

impl ProcessHandler {
    pub fn new(controller: Controller, receiver: Receiver<TimedMessage>, client: &jack::Client) -> Self {
        // Create ports
        let control_in = client.register_port("control_in", jack::MidiIn::default()).unwrap();
        let control_out = client.register_port("control_out", jack::MidiOut::default()).unwrap();
        let midi_out = client.register_port("midi_out", jack::MidiOut::default()).unwrap();

        ProcessHandler { 
            controller, 
            receiver,
            ticks_elapsed: 0,
            was_repositioned: false,
            control_in,
            control_out: MidiOut{ port: control_out },
            midi_out: MidiOut{ port: midi_out },
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Get something representing this process cycle
        let (state, pos) = client.transport_query();
        let cycle = Cycle::new(pos, self.ticks_elapsed, self.was_repositioned, process_scope.n_frames(), state);
        // Update next ticks to keep track of absoulute ticks elapsed for note off events
        self.ticks_elapsed += cycle.ticks;
        // TODO - cycle.absolute_start hack is dirty
        self.was_repositioned = cycle.is_repositioned || cycle.absolute_start == 0;

        let mut control_messages = vec![];

        // Write midi from notification handler
        while let Ok(message) = self.receiver.try_recv() {
            control_messages.push(message);
        }

        // Control out when there's somebody listening
        control_messages.extend(self.controller.sequencer.output_static_leds());

        // Process incoming midi
        control_messages.extend(self.controller.process_midi_messages(self.control_in.iter(process_scope), client));
        let (dynamic_grid_messages, sequencer_messages) = self.controller.sequencer.output_midi(&cycle);
        control_messages.extend(dynamic_grid_messages);

        // Get cycle based control & midi
        self.control_out.write(process_scope, control_messages);
        self.midi_out.write(process_scope, sequencer_messages);

        jack::Control::Continue
    }
}

pub struct NotificationHandler {
    sender: Sender<TimedMessage>,
}

impl NotificationHandler {
    pub fn new(sender: Sender<TimedMessage>) -> Self {
        NotificationHandler {
            sender: sender,
        }
    }
}

impl jack::NotificationHandler for NotificationHandler {
    fn ports_connected(&mut self, client: &jack::Client, id_a: jack::PortId, id_b: jack::PortId, are_connected: bool) {
        let port_a = client.port_by_id(id_a).unwrap();
        let port_b = client.port_by_id(id_b).unwrap();

        // If one of our ports got connected, check what we are connected to
        if (client.is_mine(&port_a) || client.is_mine(&port_b)) && are_connected {
            self.sender.send( TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7] ) ) ).unwrap();
        }
    }
}

