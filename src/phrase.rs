
use super::{BEATS_PER_BAR, TICKS_PER_BEAT};
use super::pattern::Pattern;
use super::note::NoteOff;
use super::cycle::Cycle;
use super::message::TimedMessage;
use super::playable::Playable;
use super::message::Message;

#[derive(Clone)]
pub struct Play {
    pub pattern: usize,
    pub bar: u32,
}

pub struct Phrase {
    pub playable: Playable,
    pub plays: Vec<Play>,
}

impl Phrase {
    fn create(plays: Vec<Play>) -> Self {
        Phrase { playable: Playable::new(4, 4), plays, }
    }

    pub fn new() -> Self {
        Phrase::create(vec![])
    }
    
    pub fn default() -> Self {
        Phrase::create(vec![
            Play { pattern: 0, bar: 0 },
            Play { pattern: 0, bar: 1 },
            Play { pattern: 0, bar: 2 },
            Play { pattern: 0, bar: 3 },
        ])
    }

    pub fn draw_phrase(&mut self) -> Vec<Message> {
        let grid = &mut self.playable.main_grid;
        let leds_per_bar = 8 * self.playable.zoom / self.playable.bars as u32;
        let offset = grid.width as u32 * self.playable.offset;

        self.plays.iter()
            .map(|play| {
                let absolute_led = play.bar as i32 * leds_per_bar as i32;
                let x = absolute_led as i32 - offset as i32;
                let y = play.pattern as i32;

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
}
