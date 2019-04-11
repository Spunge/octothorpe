


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

        /*
    if ( new_pos || ! _done )
    {
        pos->valid = JackPositionBBT;
        pos->beats_per_bar = transport._master_beats_per_bar;
        pos->ticks_per_beat = 1920.0;                           /* magic number means what? */               
        pos->beat_type = transport._master_beat_type;
        pos->beats_per_minute = transport._master_beats_per_minute;

        double wallclock = (double)pos->frame / (pos->frame_rate * 60);
    
        unsigned long abs_tick = wallclock * pos->beats_per_minute * pos->ticks_per_beat;                    
        unsigned long abs_beat = abs_tick / pos->ticks_per_beat;

        pos->bar = abs_beat / pos->beats_per_bar;
        pos->beat = abs_beat - (pos->bar * pos->beats_per_bar) + 1;
        pos->tick = abs_tick - (abs_beat * pos->ticks_per_beat);
        pos->bar_start_tick = pos->bar * pos->beats_per_bar * pos->ticks_per_beat;                           
        pos->bar++;
    
        _done = true; 
    }
    else
    {
        pos->tick += nframes * pos->ticks_per_beat * pos->beats_per_minute / (pos->frame_rate * 60);         

        while ( pos->tick >= pos->ticks_per_beat )
        {
            pos->tick -= pos->ticks_per_beat;

            if ( ++pos->beat > pos->beats_per_bar )
            {
                pos->beat = 1;

                ++pos->bar;

                pos->bar_start_tick += pos->beats_per_bar * pos->ticks_per_beat;
            }
        }
    }
    */

// Most of this has been shamelessly copied from non-sequencer
impl jack::TimebaseHandler for Transport {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, n_frames: jack::Frames, mut pos: jack::Position, is_new_pos: bool) {
        // Only update timebase when we are asked for it, or when our state changed
        if is_new_pos || ! self.is_up_to_date {
            pos.beats_per_bar = self.beats_per_bar as f32;
            pos.ticks_per_beat = 1920.0;
            pos.beat_type = self.beat_type as f32;
            pos.beats_per_minute = self.beats_per_minute;

            let wallclock: f64 = (pos.frame / (pos.frame_rate * 60)) as f64;

            println!("{:?}", pos.frame);

            let abs_tick: i32 = (wallclock * pos.beats_per_minute * pos.ticks_per_beat) as i32;
            let abs_beat: i32 = abs_tick / pos.ticks_per_beat as i32;

            pos.bar = abs_beat / pos.beats_per_bar as i32;
            pos.beat = abs_beat - (pos.bar * pos.beats_per_bar as i32) + 1;
            pos.tick = abs_tick - (abs_beat * pos.ticks_per_beat as i32);
            pos.bar_start_tick = (pos.bar as f32 * pos.beats_per_bar * pos.ticks_per_beat as f32) as f64;                           
            pos.bar += 1;

            self.is_up_to_date = true;
        } else {
            // TODO fix this stuff
            pos.tick += (n_frames as f64 * pos.ticks_per_beat * pos.beats_per_minute / (pos.frame_rate * 60) as f64) as i32;         

            /*
            while pos.tick >= pos.ticks_per_beat as i32
            {
                pos.tick -= pos.ticks_per_beat as i32;

                if pos.beat + 1 > pos.beats_per_bar as i32
                {
                    pos.beat = 1;

                    pos.bar += 1;

                    pos.bar_start_tick += pos.beats_per_bar as f64 * pos.ticks_per_beat;
                }
            }
            */
        }
    }
}
