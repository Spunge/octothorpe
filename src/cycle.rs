
use super::TickRange;

pub struct ProcessCycle<'a> {
    pub scope: &'a jack::ProcessScope,
    pub tick_range: TickRange,
    pub time_stop: u64,
    pub time_start: u64,
    pub is_rolling: bool,
}

/*
 * This represents a timeframe for which we will have to process midi events
 */
impl<'a> ProcessCycle<'a> {
    pub fn frame_to_tick(pos: jack::Position, frame: u32) -> f64 {
        let second = frame as f64 / pos.frame_rate as f64;
        second / 60.0 * pos.beats_per_minute * pos.ticks_per_beat
    }

    // Save client as we pass this cycle everywhere
    pub fn new(client: &'a jack::Client, scope: &'a jack::ProcessScope) -> Self {
        let cycle_times = scope.cycle_times().unwrap();
        let (state, pos) = client.transport_query();

        Self {
            scope,
            time_start: cycle_times.current_usecs,
            time_stop: cycle_times.next_usecs,
            tick_range: TickRange { 
                start: Self::frame_to_tick(pos, pos.frame) as u32,
                stop: Self::frame_to_tick(pos, pos.frame + scope.n_frames()) as u32,
            },
            is_rolling: state == 1,
        }
    }

    // Get time in usecs that this cycle lasts
    pub fn usecs(&self) -> u64 {
        self.time_stop - self.time_start
    }

    // Get time in ticks that this cycle lasts
    pub fn ticks(&self) -> u32 {
        self.tick_range.stop - self.tick_range.start
    }

    pub fn frame_to_time(&self, frame: u32) -> u64 {
        // TODO - When can this error?
        let usecs_per_frame = self.usecs() as f32 / self.scope.n_frames() as f32;
        let usecs_since_period_start = frame as f32 * usecs_per_frame;
        self.time_start + usecs_since_period_start as u64
    }

    // TODO - This can panic, is that what we want?
    pub fn tick_to_frame(&self, tick: u32) -> u32 {
        let tick_in_cycle = tick - self.tick_range.start;
        let frame_in_cycle = tick_in_cycle as f64 / self.ticks() as f64 * self.scope.n_frames() as f64;
        frame_in_cycle as u32
    }
}

