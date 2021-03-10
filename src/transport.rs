
pub struct Transport {
    pub beats_per_minute: f64,
    pub beats_per_bar: f32,
    pub beat_type: f32,
}

impl Transport {
    pub const TICKS_PER_BEAT: f64 = 1920.0;

    pub fn new() -> Self {
        Self {
            beats_per_minute: 137.0,
            beats_per_bar: 4.0,
            beat_type: 4.0,
        }
    }
}
