
use super::pattern::Pattern;
use super::phrase::Phrase;
use super::handlers::Writer;
use super::cycle::Cycle;
use super::note::NoteOff;
use super::TICKS_PER_BEAT;

enum View {
    Pattern,
    Phrase,
}

pub struct Instrument {
    patterns: Vec<Pattern>,
    phrases: Vec<Phrase>,
    playing_phrase: usize,
    showing_phrase: usize,
    showing_pattern: usize,

    view: View,

    note_offs: Vec<NoteOff>,
    channel: u8,
}

impl Instrument {
    fn create(channel: u8, patterns: Vec<Pattern>, phrases: Vec<Phrase>) -> Self {
        Instrument {
            patterns,
            phrases,
            view: View::Pattern,
            playing_phrase: 0,
            showing_phrase: 0,
            showing_pattern: 0,

            note_offs: vec![],
            channel,
        }
    }
    
    pub fn new(channel: u8) -> Self {
        Instrument::create(channel, vec![Pattern::new()], vec![Phrase::new()])
    }

    pub fn default(channel: u8) -> Self {
        Instrument::create(channel, vec![Pattern::default()], vec![Phrase::default()]) 
    }

    // Clear midi controller grids
    pub fn clear(&mut self, writer: &mut Writer) {
        self.patterns[self.showing_pattern].clear(writer)
    }

    // Draw this instrument grids
    pub fn draw(&mut self, cycle: &Cycle, was_repositioned: bool, writer: &mut Writer) {
        let pattern = &mut self.patterns[self.showing_pattern];

        match self.view {
            View::Pattern => {
                // Clean grid on starting
                if cycle.absolute_start == 0 {
                    pattern.clear(writer);
                    pattern.draw_pattern(writer);
                }

                if was_repositioned {
                    let beat_start = (cycle.start / TICKS_PER_BEAT as u32) * TICKS_PER_BEAT as u32;
                    let reposition_cycle = cycle.repositioned(beat_start);

                    pattern.draw_indicator(&reposition_cycle, writer);
                }

                // Update grid when running, after repositioning
                if cycle.is_rolling {
                    pattern.draw_indicator(cycle, writer);
                }
            },
            View::Phrase => {
                println!("phrase view todo");
            },
        }
    }

    pub fn output_note_offs(&mut self, cycle: &Cycle, writer: &mut Writer) {
        let channel = self.channel;

        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    writer.write(note_off.note.note_off(frames, channel));
                    false
                },
                None => true
            }
        });
    }

    // Output midi
    pub fn output(&mut self, cycle: &Cycle, writer: &mut Writer) {
        self.output_note_offs(cycle, writer);

        if cycle.is_rolling {
            // Get note offs by playing note_ons
            self.note_offs.append(&mut self.phrases[self.playing_phrase]
                                  .output_notes(cycle, self.channel, &self.patterns, writer));

            // Put same key notes next to each other
            self.note_offs.sort();
            self.note_offs.reverse();
            self.note_offs.dedup_by(|a, b| { a.note.key == b.note.key });
        }
    }
}
