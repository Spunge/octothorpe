
use super::TickRange;

// All the things we can show in grid
pub trait LoopableEvent: Clone + std::fmt::Debug {
    fn start(&self) -> u32;
    fn stop(&self) -> Option<u32>;
    fn set_stop(&mut self, stop: u32);
    fn set_start(&mut self, start: u32);
    fn is_on_row(&self, index: u8) -> bool;
    fn is_on_same_row(&self, other: &Self) -> bool;
    fn row(&self, offset: u8) -> u8;

    fn is_looping(&self) -> bool {
        match self.stop() {
            Some(stop) => stop <= self.start(),
            _ => false
        }
    }
    
    fn length(&self, container_length: u32) -> u32 {
        let mut stop = self.stop().unwrap();

        if self.is_looping() {
            stop += container_length;
        }

        stop - self.start()
    }

    fn overlaps_tick_range(&self, start: u32, stop: u32) -> bool {
        if let Some(self_stop) = self.stop() {
            if self.is_looping() {
                // is not not contained
                ! (start >= self_stop && stop <= self.start())
            } else {
                self.start() < stop && self_stop > start
            }
        } else {
            false
        }
    }

    // Does this event contain another event wholly?
    fn contains(&self, other: &impl LoopableEvent, max_length: u32) -> bool {
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
    fn resize_to_fit(&mut self, other: &impl LoopableEvent, max_length: u32) -> Option<Self> {
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
#[derive(Debug, Clone, Copy)]
pub struct LoopableNoteEvent {
    pub note: u8,
    pub start: u32,
    pub start_velocity: u8,
    pub stop: Option<u32>,
    pub stop_velocity: Option<u8>,
}

impl LoopableEvent for LoopableNoteEvent {
    fn start(&self) -> u32 { self.start }
    fn stop(&self) -> Option<u32> { self.stop }
    fn set_start(&mut self, tick: u32) { self.start = tick }
    fn set_stop(&mut self, tick: u32) { self.stop = Some(tick) }
    fn is_on_row(&self, index: u8) -> bool { self.note == index }
    fn is_on_same_row(&self, other: &Self) -> bool { self.note == other.note }
    fn row(&self, offset: u8) -> u8 { self.note - offset }
}

impl LoopableNoteEvent {
    pub fn new(start: u32, note: u8, start_velocity: u8) -> Self {
        Self { start, note, start_velocity, stop: None, stop_velocity: None }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoopablePatternEvent {
    pub start: u32,
    pub stop: Option<u32>,
    pub pattern: u8,
}

impl LoopableEvent for LoopablePatternEvent {
    fn start(&self) -> u32 { self.start }
    fn stop(&self) -> Option<u32> { self.stop }
    fn set_start(&mut self, tick: u32) { self.start = tick }
    fn set_stop(&mut self, tick: u32) { self.stop = Some(tick) }
    fn is_on_row(&self, index: u8) -> bool { self.pattern == index }
    fn is_on_same_row(&self, other: &Self) -> bool { self.pattern == other.pattern }
    fn row(&self, offset: u8) -> u8 { self.pattern - offset }
}

impl LoopablePatternEvent {
    pub fn new(start: u32, pattern: u8) -> Self {
        LoopablePatternEvent { start, stop: None, pattern, }
    }

    pub fn absolute_tick_ranges(&self, phrase_length: u32) -> Vec<(TickRange, u32, u8)> {
        if self.is_looping() {
            let offset = phrase_length - self.start();
            vec![
                (TickRange::new(0, self.stop().unwrap()), offset, self.pattern), 
                (TickRange::new(self.start(), phrase_length), 0, self.pattern)
            ]
        } else {
            vec![(TickRange::new(self.start(), self.stop().unwrap()), 0, self.pattern)]
        }
    }
}

// We also keep start around so we can use this for different note visualizations aswell
#[derive(Debug)]
pub struct PlayingNoteEvent {
    pub start: u32,
    pub stop: u32,
    pub note: u8,
    pub start_velocity: u8,
    pub stop_velocity: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new(start: u32, stop: Option<u32>) -> LoopablePatternEvent {
        LoopablePatternEvent { start, stop, pattern: 0 }
    }

    #[test]
    fn contains() {
        let no_end = tests::new(0, None);
        let normal = tests::new(50, Some(150));
        let looping = tests::new(150, Some(50));

        assert_eq!(no_end.contains(&normal, 200), false);
        assert_eq!(no_end.contains(&looping, 200), false);
        assert_eq!(normal.contains(&looping, 200), false);
        assert_eq!(normal.contains(&no_end, 200), false);
        assert_eq!(normal.contains(&tests::new(50, Some(100)), 200), true);
        assert_eq!(normal.contains(&tests::new(100, Some(150)), 200), true);
        assert_eq!(normal.contains(&tests::new(50, Some(150)), 200), true);
        assert_eq!(normal.contains(&tests::new(50, Some(150)), 200), true);
        assert_eq!(looping.contains(&tests::new(50, Some(150)), 200), false);
        assert_eq!(looping.contains(&tests::new(150, Some(170)), 200), true);
        assert_eq!(looping.contains(&tests::new(20, Some(50)), 200), true);
        assert_eq!(looping.contains(&tests::new(160, Some(40)), 200), true);
        assert_eq!(looping.contains(&tests::new(150, Some(50)), 200), true);
        assert_eq!(looping.contains(&tests::new(120, Some(50)), 200), false);
        assert_eq!(looping.contains(&tests::new(150, None), 200), false);
    }

    #[test]
    fn resize_to_fit() {
        let mut no_end = tests::new(0, None);
        let mut looping = tests::new(150, Some(50));

        let mut event = tests::new(50, Some(150));
        let split = event.resize_to_fit(&tests::new(100, Some(150)), 200);
        assert_eq!((50, Some(100)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(50, Some(150));
        let split = event.resize_to_fit(&tests::new(0, Some(30)), 200);
        assert_eq!((50, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(50, Some(150));
        let split = event.resize_to_fit(&tests::new(50, Some(100)), 200);
        assert_eq!((100, Some(150)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(100, Some(170)), 200);
        assert_eq!((170, Some(50)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(40, Some(100)), 200);
        assert_eq!((150, Some(40)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(60, Some(100)), 200);
        assert_eq!((150, Some(50)), (event.start, event.stop));
        assert_eq!(true, split.is_none());

        let mut event = tests::new(50, Some(150));
        let split = event.resize_to_fit(&tests::new(80, Some(100)), 200);
        assert_eq!((50, Some(80)), (event.start, event.stop));
        assert_eq!(Some((100, Some(150))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(20, Some(40)), 200);
        assert_eq!((150, Some(20)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(170, Some(40)), 200);
        assert_eq!((150, Some(170)), (event.start, event.stop));
        assert_eq!(Some((40, Some(50))), split.and_then(|e| Some((e.start, e.stop))));

        let mut event = tests::new(150, Some(50));
        let split = event.resize_to_fit(&tests::new(160, Some(180)), 200);
        assert_eq!((150, Some(160)), (event.start, event.stop));
        assert_eq!(Some((180, Some(50))), split.and_then(|e| Some((e.start, e.stop))));
    }
}

