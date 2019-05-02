
use super::{BEATS_PER_BAR, TICKS_PER_BEAT};
use super::pattern::Pattern;
use super::note::NoteOff;
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::playable::Playable;
use super::message::Message;

#[derive(Clone)]
pub struct PlayedPattern {
    pub index: usize,
    pub bar: u32,
}

pub struct Phrase {
    pub playable: Playable,
    pub patterns: Vec<PlayedPattern>,
}

impl Phrase {
    fn create(patterns: Vec<PlayedPattern>) -> Self {
        Phrase { playable: Playable::new(4, 4), patterns, }
    }

    pub fn new() -> Self {
        Phrase::create(vec![])
    }
    
    pub fn default() -> Self {
        Phrase::create(vec![
            PlayedPattern { index: 0, bar: 0 },
            PlayedPattern { index: 0, bar: 1 },
            PlayedPattern { index: 0, bar: 2 },
            PlayedPattern { index: 0, bar: 3 },
        ])
    }

    pub fn draw_phrase(&mut self) -> Vec<Message> {
        let grid = &mut self.playable.main_grid;
        let leds_per_bar = 8 * self.playable.zoom / self.playable.bars as u32;
        let offset = grid.width as u32 * self.playable.offset;

        self.patterns.iter()
            .map(|pattern| {
                let absolute_led = pattern.bar as i32 * leds_per_bar as i32;
                let x = absolute_led as i32 - offset as i32;
                let y = pattern.index as i32;

                let head = (x, y, 1);
                let tail: Vec<(i32, i32, u8)> = (1..leds_per_bar).map(|led| (x + led as i32, y, 5)).collect();

                let mut messages = vec![head];
                messages.extend(tail);
                messages
            })
            .flatten()
            .filter_map(|led| {
                let (x, y, state) = led;
                grid.try_switch_led(x, y, state)
            })
            .collect()
    }

    pub fn draw(&mut self) -> Vec<Message> {
        vec![ 
            self.draw_phrase(),
            self.playable.draw_length(),
            self.playable.draw_zoom() 
        ].into_iter().flatten().collect()
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        vec![ 
            self.playable.main_grid.clear(force), 
            self.playable.length_grid.clear(force),
            self.playable.zoom_grid.clear(force) 
        ].into_iter().flatten().collect()
    }

    pub fn playing_notes(&self, cycle: &Cycle, patterns: &[Pattern]) -> Vec<(TimedMessage, NoteOff)> {
        let ticks_per_bar = BEATS_PER_BAR as u32 * TICKS_PER_BEAT as u32;
        let phrase_ticks = ticks_per_bar * self.playable.bars as u32;
        
        self.patterns.iter()
            .filter_map(|pattern| {
                let pattern_ticks = patterns[pattern.index].playable.bars as u32 * ticks_per_bar;
                let start = pattern.bar * ticks_per_bar;
                let end = start + pattern_ticks;
                let cycle_start = cycle.start % phrase_ticks;
                let cycle_end = cycle_start + cycle.ticks;

                // Is pattern playing?
                if start < cycle_end && end > cycle_start {
                    let notes = patterns[pattern.index].notes.iter()
                        .filter_map(move |note| {
                            let note_start = note.tick + start;

                            // Does note fall in cycle?
                            if note_start >= cycle_start && note_start < cycle_end {
                                let delta_ticks = note_start - cycle_start;
                                let delta_frames = (delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;

                                let message = TimedMessage::new(delta_frames, note.message());
                                let note_off = note.note_off(cycle.absolute_start + delta_ticks);

                                Some((message, note_off ))
                            } else {
                                None
                            }
                        });
                    
                    Some(notes)
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}
