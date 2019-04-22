

use jack_sys as j;

pub struct Transport {
    beats_per_minute: f64,
    beats_per_bar: isize,
    beat_type: isize,
    is_up_to_date: bool,
}

impl Transport {
    pub fn new(beats_per_minute: f64) -> Self {
        Transport {
            is_up_to_date: false,
            beats_per_minute: beats_per_minute,
            beats_per_bar: 4,
            beat_type: 4,
        }
    }
}

impl jack::TimebaseHandler for Transport {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
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
