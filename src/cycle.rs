
#[derive(Clone, Debug)]
pub struct Cycle {
    pub start: u32,
    pub end: u32,
    pub absolute_start: u32,
    pub absolute_end: u32,
    pub ticks: u32,
    pub frames: u32,
    pub is_rolling: bool,
    pub is_repositioned: bool,
}

impl Cycle {
    pub fn new(pos: jack::Position, absolute_start: u32, frames: u32, state: u32) -> Self {
        let start = Cycle::get_tick(pos, pos.frame) as u32;
        let end = Cycle::get_tick(pos, pos.frame + frames) as u32;
        let ticks = end - start;

        let is_rolling = state == 1;
        // Seems repositioning causes a 0 ticks cycle
        let is_repositioned = start == end;

        Cycle { 
            start, 
            end, 
            absolute_start,
            absolute_end: absolute_start + ticks,
            ticks, 
            frames, 
            is_rolling,
            is_repositioned,
        }
    }

    // Used to get repositioned cycle for transport reposition logic
    pub fn repositioned(&self, start: u32) -> Self {
        let mut cycle = self.clone();
        cycle.start = start;
        cycle.end = start + cycle.ticks;
        cycle
    }

    pub fn get_tick(pos: jack::Position, frame: u32) -> f64 {
        let second = frame as f64 / pos.frame_rate as f64;
        second / 60.0 * pos.beats_per_minute * pos.ticks_per_beat
    }

    pub fn contains(&self, tick: u32) -> bool {
        tick >= self.start && tick < self.end 
    }

    pub fn contains_recurring(&self, tick: u32, interval: u32) -> bool {
        let pattern_start = self.start % interval;
        let pattern_end = pattern_start + self.ticks;
        let next_tick = tick + interval;

        tick >= pattern_start && tick < pattern_end
            || next_tick >= pattern_start && next_tick < pattern_end
    }

    pub fn contains_absolute(&self, tick: u32) -> bool {
        tick >= self.absolute_start && tick < self.absolute_end 
    }

    pub fn delta_ticks(&self, tick: u32) -> u32 {
        tick - self.start
    }

    pub fn delta_ticks_absolute(&self, absolute_tick: u32) -> u32 {
        absolute_tick - self.absolute_start
    }

    pub fn delta_ticks_recurring(&self, tick: u32, interval: u32) -> u32 {
        let pattern_start = self.start % interval;

        if pattern_start > tick {
            tick + interval - pattern_start
        } else {
            tick - pattern_start
        }
    }

    fn ticks_to_frames(&self, ticks: u32) -> u32 {
        (ticks as f64 / self.ticks as f64 * self.frames as f64) as u32
    }

    pub fn delta_frames(&self, tick: u32) -> u32 {
        self.ticks_to_frames(self.delta_ticks(tick))
    }

    pub fn delta_frames_absolute(&self, tick: u32) -> u32 {
        self.ticks_to_frames(self.delta_ticks_absolute(tick))
    }

    pub fn delta_frames_recurring(&self, tick: u32, interval: u32) -> u32 {
        self.ticks_to_frames(self.delta_ticks_recurring(tick, interval))
    }
}

