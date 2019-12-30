
// All the things we can show in grid
pub trait Loopable: Clone {
    fn start(&self) -> u32;
    fn stop(&self) -> Option<u32>;
    fn set_stop(&mut self, stop: u32);
    fn set_start(&mut self, start: u32);

    fn is_looping(&self) -> bool {
        match self.stop() {
            Some(stop) => stop <= self.start(),
            _ => false
        }
    }

    // Does this event contain another event wholly?
    fn contains(&self, other: &impl Loopable, max_length: u32) -> bool {
        match (self.stop(), other.stop()) {
            (Some(self_stop), Some(other_stop)) => {
                if ! self.is_looping() && ! other.is_looping() || self.is_looping() && other.is_looping() {
                    // Normal ranges both
                    self.start() <= other.start() && self_stop >= other_stop
                } else {
                    if self.is_looping() && ! other.is_looping() {
                        self.start() <= other.start() || self_stop >= other_stop 
                    } else {
                        // Normal range can only truly contain a looping range when it's as long as the container
                        self.start() == 0 && self_stop == max_length
                    }
                }
            },
            _ => false,
        }
    }

    // Move out of the way of other event
    fn resize_to_fit(&mut self, other: &impl Loopable, max_length: u32) -> Option<Self> {
        match (self.stop(), other.stop()) {
            (Some(self_stop), Some(other_stop)) => {
                let starts_before = self.start() < other.start();
                let stops_before = self_stop <= other_stop;

                let stops_after = self_stop > other_stop;
                let starts_after = self.start() >= other.start();

                //       [    other   ]
                // [    self    ]              
                let end_overlaps = self_stop > other.start() && (stops_before || starts_before);
                // [    other   ]
                //       [    self    ]
                let begin_overlaps = self.start() < other_stop && (stops_after || starts_after);
    
                match (begin_overlaps, end_overlaps) {
                    // Only begin overlaps
                    (true, false) => { self.set_start(other_stop); None },
                    // Only end overlaps
                    (false, true) => { self.set_stop(other.start()); None },
                    // They both overlap || don't overlap
                    // Could be valid note placement || split, depending on looping or not
                    _ => {
                        if self.contains(other, max_length) {
                            // Create split pattern event
                            let mut split = self.clone();
                            split.set_start(other_stop);
                            //let event = PatternEvent::new(other_stop, self.stop, self.pattern);
                            // Adjust own event
                            self.set_stop(other.start());
                            //self.stop = Some(other.start);
                            Some(split)
                        } else {
                            None
                        }
                    },
                }
            },
            _ => None,
        }
    }
}

// note, velocity
#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub note: u8,
    pub start: u32,
    pub start_velocity: u8,
    pub stop: Option<u32>,
    pub stop_velocity: Option<u8>,
}

impl Loopable for NoteEvent {
    fn start(&self) -> u32 { self.start }
    fn stop(&self) -> Option<u32> { self.stop }
    fn set_start(&mut self, tick: u32) { self.start = tick }
    fn set_stop(&mut self, tick: u32) { self.stop = Some(tick) }
}

impl NoteEvent {
    pub fn new(start: u32, note: u8, start_velocity: u8) -> Self {
        Self { start, note, start_velocity, stop: None, stop_velocity: None }
    }
}

#[derive(Debug, Clone)]
pub struct PatternEvent {
    pub start: u32,
    pub stop: Option<u32>,
    pub pattern: usize,
}

impl Loopable for PatternEvent {
    fn start(&self) -> u32 { self.start }
    fn stop(&self) -> Option<u32> { self.stop }
    fn set_start(&mut self, tick: u32) { self.start = tick }
    fn set_stop(&mut self, tick: u32) { self.stop = Some(tick) }
}

impl PatternEvent {
    pub fn new(start: u32, stop: Option<u32>, pattern: usize) -> Self {
        PatternEvent { start, stop, pattern, }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains() {
        let no_end = PatternEvent::new(0, None, 0);
        let normal = PatternEvent::new(50, Some(150), 0);
        let looping = PatternEvent::new(150, Some(50), 0);

        assert_eq!(no_end.contains(&normal, 200), false);
        assert_eq!(no_end.contains(&looping, 200), false);
        assert_eq!(normal.contains(&looping, 200), false);
        assert_eq!(normal.contains(&no_end, 200), false);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(100), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(100, Some(150), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(150), 0), 200), true);
        assert_eq!(normal.contains(&PatternEvent::new(50, Some(150), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(50, Some(150), 0), 200), false);
        assert_eq!(looping.contains(&PatternEvent::new(150, Some(170), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(20, Some(50), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(160, Some(40), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(150, Some(50), 0), 200), true);
        assert_eq!(looping.contains(&PatternEvent::new(120, Some(50), 0), 200), false);
        assert_eq!(looping.contains(&PatternEvent::new(150, None, 0), 200), false);
    }

    #[test]
    fn resize_to_fit() {
        let mut no_end = PatternEvent::new(0, None, 0);
        let mut looping = PatternEvent::new(150, Some(50), 0);

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(100, Some(150), 0), 200);
        assert_eq!((50, Some(100)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(0, Some(30), 0), 200);
        assert_eq!((50, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(50, Some(100), 0), 200);
        assert_eq!((100, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(100, Some(170), 0), 200);
        assert_eq!((170, Some(50)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(40, Some(100), 0), 200);
        assert_eq!((150, Some(40)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(60, Some(100), 0), 200);
        assert_eq!((150, Some(50)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = PatternEvent::new(50, Some(150), 0);
        let split = event.resize_to_fit(&PatternEvent::new(80, Some(100), 0), 200);
        assert_eq!((50, Some(80)), (event.start, event.stop));
        assert_eq!(Some((100, Some(150))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(20, Some(40), 0), 200);
        assert_eq!((150, Some(20)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(170, Some(40), 0), 200);
        assert_eq!((150, Some(170)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = PatternEvent::new(150, Some(50), 0);
        let split = event.resize_to_fit(&PatternEvent::new(160, Some(180), 0), 200);
        assert_eq!((150, Some(160)), (event.start, event.stop));
        assert_eq!(Some((180, Some(50))), split.and_then(|e| Some((e.start, e.stop))));
    }
}

