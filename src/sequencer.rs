
use super::TICKS_PER_BEAT;
use super::handlers::{Cycle, Writer};
use super::message::Message;

struct Indicator {
    active_led: Option<u8>,
}

impl Indicator {
    fn get_note(&self, time: u32, state: u8) -> Message {
        Message::Note(time, [0x90 + self.active_led.unwrap() , 0x34, state])
    }

    fn output(&mut self, cycle: Cycle, control_out: &mut Writer, pattern_length: u32) {
        // This indicator just initialized, activate current beat led NOW
        if self.active_led.is_none() {
            self.set_active_led(cycle.get_abs_beat());
            control_out.write(self.get_note(0, 1));
        }

        //if(cycle.pos.tick == 0);

        // Will we be in next beat by the end of this process cycle?
        /*
        if cycle.transport_is_rolling && next_beat != current_beat {
            let ticks_in_cycle = cycle.end_tick - cycle.start_tick;

            let ticks_left_in_beat = ticks_in_cycle - (cycle.end_tick % TICKS_PER_BEAT);

            println!("{:?}", cycle.start_tick);
            println!("{:?}", ticks_left_in_beat);
            println!("{:?}\n", ticks_left_in_beat / ticks_in_cycle);
            //println!("{:?} {:?}", current_beat, next_beat);
            
            // Switch active led off
            control_out.write(self.get_note(0, 0));

            // Switch next led on
            self.set_active_led(next_beat);
            control_out.write(self.get_note(0, 1));
        }
        */
    }
}

#[derive(Debug)]
struct Note {
    // Key in MIDI
    key: u32,
    // Length in ticks
    length: u32,
}

impl Note {
    fn default() -> Self {
        Note {
            // A4
            key: 69,
            // Quarter beat
            length: (TICKS_PER_BEAT / 4 as f64) as u32,
        }
    }
}

#[derive(Debug)]
struct NoteEvent {
    tick: u32,
    note: Note,
}

#[derive(Debug)]
struct Pattern {
    // Length in beats
    length: u32,
    notes: Vec<NoteEvent>,
}

pub struct Sequencer {
    pattern: Pattern,
    indicator: Indicator,
}

impl Sequencer {
    pub fn new() -> Self {
        Sequencer{
            indicator: Indicator::new(),
            pattern: Pattern {
                length: 1,
                notes: vec![NoteEvent{ tick: 0, note: Note::default() }]
            }
        }
    }

    pub fn output(&mut self, cycle: Cycle, control_out: &mut Writer, midi_out: &mut Writer) {
        self.indicator.output(cycle, control_out, &self.pattern.length);
    }
}
