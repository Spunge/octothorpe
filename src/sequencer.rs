
use super::TICKS_PER_BEAT;
use super::handlers::Writer;
use super::message::{Message, MessageData};
use super::note::Note;
use super::cycle::Cycle;
use super::pattern::Pattern;

pub struct Indicator {
    pub leds: u32,
    active_led: u32,
}

impl Indicator {
    fn switch_led(&mut self, led: u32, state: u8, frames: u32, writer: &mut Writer) {
        if led < self.leds {
            writer.write(Message::new(frames, MessageData::Note([0x90 + led as u8, 0x34, state])));
        }
    }

    fn clear(&mut self, writer: &mut Writer) {
        (0..self.leds + 1).for_each(|led| {
            self.switch_led(led, 0, 0, writer);
        });
    }

    fn switch_to_led(&mut self, led: u32, frames: u32, writer: &mut Writer) {
        self.switch_led(self.active_led, 0, frames, writer);
        self.active_led = led;
        self.switch_led(self.active_led, 1, frames, writer);
    }

    fn draw(&mut self, cycle: &Cycle, pattern: &Pattern, writer: &mut Writer) {
        let steps = pattern.length;
        let ticks = steps * TICKS_PER_BEAT as u32;

        (0..steps).for_each(|beat| { 
            let tick = beat * TICKS_PER_BEAT as u32;

            if let Some(delta_ticks) = cycle.delta_ticks_recurring(tick, ticks) {
                self.switch_to_led(beat, cycle.ticks_to_frames(delta_ticks), writer);
            }
        })
    }
}

pub struct Sequencer {
    pattern: Pattern,
    indicator: Indicator,
    // Keep track of elapsed ticks to trigger note_off when transport stops
    was_repositioned: bool,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            was_repositioned: true,

            indicator: Indicator{
                leds: 8,
                active_led: 0,
            },

            pattern: Pattern {
                start: 0,
                length: 11,

                note_offs: Vec::new(),
                notes: vec![
                    Note::new(0, TICKS_PER_BEAT as u32, 72),
                    Note::new(TICKS_PER_BEAT as u32, TICKS_PER_BEAT as u32, 69),
                    Note::new(TICKS_PER_BEAT as u32 * 2, TICKS_PER_BEAT as u32, 69),
                    Note::new(TICKS_PER_BEAT as u32 * 3, TICKS_PER_BEAT as u32, 69),
                ],
            },
        }
    }

    pub fn update(&mut self, cycle: &Cycle) {
        // Only run reposition stuff once
        if self.was_repositioned {
            self.was_repositioned = false;
        }
        self.was_repositioned = cycle.is_repositioned;
    }

    pub fn output(&mut self, cycle: &Cycle, control_out: &mut Writer, midi_out: &mut Writer) {
        // Always turn notes off after their time is up to prevent infinite notes
        self.pattern.output_note_off_events(&cycle, midi_out);

        // Clean grid on starting
        if cycle.absolute_start == 0 {
            self.indicator.clear(control_out);
        }

        if self.was_repositioned {
            let beat_start = (cycle.start / TICKS_PER_BEAT as u32) * TICKS_PER_BEAT as u32;
            let reposition_cycle = cycle.repositioned(beat_start);

            self.indicator.draw(&reposition_cycle, &self.pattern, control_out);
        }

        // Update grid when running, after repositioning
        if cycle.is_rolling {
            self.indicator.draw(cycle, &self.pattern, control_out);
        }

        if cycle.is_rolling {
            self.pattern.output_note_on_events(cycle, midi_out);
        }

        self.update(cycle);
    }
}
