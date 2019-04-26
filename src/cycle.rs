
#[derive(Debug)]
pub struct Cycle {
    pub start: u32,
    pub end: u32,
    pub ticks: u32,
    pub frames: u32,
    pub is_rolling: bool,
}

impl Cycle {
    pub fn repositioned(&self, start: u32) -> Self {
        Cycle {
            start,
            end: start + self.ticks, 
            ticks: self.ticks,
            frames: self.frames,
            is_rolling: self.is_rolling,
        }
    }

    pub fn new(pos: jack::Position, frames: u32, state: u32) -> Self {
        let start = Cycle::get_tick(pos, pos.frame) as u32;
        let end = Cycle::get_tick(pos, pos.frame + frames) as u32;

        Cycle { 
            start,
            end,
            ticks: end - start,
            frames,
            is_rolling: state == 1,
        }
    }

    pub fn get_tick(pos: jack::Position, frame: u32) -> f64 {
        let second = frame as f64 / pos.frame_rate as f64;
        second / 60.0 * pos.beats_per_minute * pos.ticks_per_beat
    }

    pub fn contains(&self, tick: u32) -> bool {
        tick >= self.start && tick < self.end 
    }

    pub fn ticks_till_tick(&self, tick: u32) -> u32 {
        tick - self.start
    }

    pub fn frames_till_tick(&self, tick: u32) -> u32 {
        (self.ticks_till_tick(tick) as f64 / self.ticks as f64 * self.frames as f64) as u32
    }
}

