
// We need jack_sys C library as timebase master logic is not implemented in rust-jack crate
use jack_sys;
use crate::*;

pub struct TimebaseHandler {
    transport: Arc<Mutex<Transport>>,
}

impl TimebaseHandler {

    pub fn new(transport: Arc<Mutex<Transport>>) -> Self {
        TimebaseHandler {
            // TODO - Put this in transport struct that we can share via arc/mutex
            transport,
        }
    }
}

impl jack::TimebaseHandler for TimebaseHandler {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, _n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
        unsafe {
            // Set position type
            (*pos).valid = jack_sys::JackPositionBBT;

            // BPM changed?
            //if ! is_new_pos && (*pos).beats_per_minute != self.beats_per_minute {
                //println!("{:?}", (*pos).beats_per_minute);
            //}

            let transport = self.transport.lock().unwrap();

            // Update timebase information
            (*pos).beats_per_bar = transport.beats_per_bar;
            (*pos).ticks_per_beat = Transport::TICKS_PER_BEAT;
            (*pos).beat_type = transport.beat_type;
            (*pos).beats_per_minute = transport.beats_per_minute;

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


