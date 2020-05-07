
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;
use jack_sys as j;

pub mod controller;
pub mod message;
pub mod sequencer;
pub mod cycle;
pub mod track;
pub mod loopable;
pub mod sequence;
pub mod surface;
pub mod port;
pub mod mixer;
pub mod events;
pub mod instrument;

use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use sequencer::Sequencer;
use controller::*;
use mixer::*;
use surface::Surface;
use cycle::*;

#[derive(Copy, Clone, Debug)]
pub struct TickRange {
    pub start: u32,
    pub stop: u32,
}

impl TickRange {
    fn new(start: u32, stop: u32) -> Self {
        Self { start, stop }
    }

    pub fn plus(&self, delta: u32) -> Self {
        Self {
            start: self.start + delta,
            stop: self.stop + delta,
        }
    }

    pub fn minus(&self, delta: u32) -> Self {
        Self {
            start: self.start - delta,
            stop: self.stop - delta,
        }
    }

    pub fn contains(&self, tick: u32) -> bool {
        tick >= self.start && tick < self.stop
    }

    pub fn overlaps(&self, other: &TickRange) -> bool {
        self.start < other.stop && self.stop > other.start
    }

    pub fn length(&self) -> u32 {
        self.stop - self.start
    }
}

pub struct TimebaseHandler {
    beats_per_minute: f64,
    beats_per_bar: f32,
    beat_type: f32,
    is_up_to_date: bool,

    receiver: Receiver<f64>,
}

impl TimebaseHandler {
    pub const TICKS_PER_BEAT: f64 = 1920.0;

    pub fn new(receiver: Receiver<f64>) -> Self {
        TimebaseHandler {
            beats_per_minute: 137.0,
            is_up_to_date: false,
            beats_per_bar: 4.0,
            beat_type: 4.0,
            receiver,
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
                (*pos).beats_per_bar = self.beats_per_bar;
                (*pos).ticks_per_beat = Self::TICKS_PER_BEAT;
                (*pos).beat_type = self.beat_type;
                (*pos).beats_per_minute = self.beats_per_minute;

                self.is_up_to_date = true;
            }

            let abs_tick = ProcessCycle::frame_to_tick(*pos, (*pos).frame);
            let abs_beat = abs_tick / (*pos).ticks_per_beat;

            // Plus 1 as humans tend not to count from 0
            (*pos).bar = (abs_beat / (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).beat = (abs_beat % (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).bar_start_tick = (abs_beat as i32 * (*pos).ticks_per_beat as i32) as f64;
            (*pos).tick = abs_tick as i32 - (*pos).bar_start_tick as i32;
        }
    }
}


pub struct ProcessHandler {
    // Controllers
    apc20: APC20,
    apc40: APC40,

    mixer: Mixer,
    sequencer: Sequencer,
    surface: Surface,
}

impl ProcessHandler {
    pub fn new(
        _timebase_sender: Sender<f64>,
        client: &jack::Client
    ) -> Self {
        ProcessHandler { 
            apc20: APC20::new(client),
            apc40: APC40::new(client),

            mixer: Mixer::new(client),
            sequencer: Sequencer::new(client), 
            surface: Surface::new(),
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, client: &jack::Client, scope: &jack::ProcessScope) -> jack::Control {
        // Get something representing this process cycle
        let cycle = ProcessCycle::new(client, scope);

        self.apc20.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface, &mut self.mixer);
        self.apc40.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface, &mut self.mixer);

        self.sequencer.autoqueue_next_sequence(&cycle);

        // Sequencer first at it will cache playing notes, these we can use for sequence visualization
        self.sequencer.output_midi(&cycle);
        self.mixer.output_midi(&cycle);

        self.apc20.output_midi(&cycle, &mut self.sequencer, &mut self.surface);
        self.apc40.output_midi(&cycle, &mut self.sequencer, &mut self.surface);

        jack::Control::Continue
    }
}


fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("Octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (timebase_sender, timebase_receiver) = channel();

    let processhandler = ProcessHandler::new(timebase_sender, &client);
    let timebasehandler = TimebaseHandler::new(timebase_receiver);

    // Activate client
    let _async_client = client
        .activate_async((), processhandler, timebasehandler)
        .unwrap();

    // Wait for user to input string
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input).ok();
}

