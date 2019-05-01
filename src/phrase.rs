
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
    pub fn new() -> Self {
        Phrase {
            playable: Playable::new(4, 4),
            plays: vec![
                Play { pattern: 0, bar: 0 },
                Play { pattern: 0, bar: 1 },
                Play { pattern: 0, bar: 2 },
                Play { pattern: 0, bar: 3 },
            ],
        }
    }

    pub fn redraw(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        messages.extend(self.draw());
        messages
    }

    pub fn draw_phrase(&mut self) -> Vec<Message> {
        vec![]
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
