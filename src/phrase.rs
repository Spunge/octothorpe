
use super::TICKS_PER_BEAT;
use super::pattern::Pattern;
use super::note::NoteOff;
use super::cycle::Cycle;
use super::message::TimedMessage;

#[derive(Clone)]
struct Play {
    pattern: usize,
    bar: u32,
}

pub struct Phrase {
    bars: u32,
    beats_per_bar: u32,
    plays: Vec<Play>,
}

impl Phrase {
    pub fn new() -> Self {
        Phrase {
            bars: 4,
            beats_per_bar: 4,
            plays: vec![],
        }
    }

    pub fn default() -> Self {
        Phrase {
            bars: 4,
            beats_per_bar: 4,
            plays: vec![
                Play{ pattern: 0, bar: 0 },
                Play{ pattern: 0, bar: 1 },
                Play{ pattern: 0, bar: 2 },
                Play{ pattern: 0, bar: 3 },
            ],
        }
    }

    pub fn note_on_messages(&self, cycle: &Cycle, channel: u8, patterns: &Vec<Pattern>, note_offs: &mut Vec<NoteOff>) -> Vec<TimedMessage> {
        let ticks_per_bar = self.beats_per_bar * TICKS_PER_BEAT as u32;
        let ticks = self.bars * ticks_per_bar;

        self.plays.iter()
            // Is play located within phrase?
            .filter(|play| { play.bar < self.bars })
            // Play pattern
            .flat_map(|play| {
                patterns[play.pattern].note_on_messages(cycle, channel, play.bar * ticks_per_bar, ticks, note_offs)
            })
            .collect()
    }
}
