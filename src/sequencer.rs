
use super::handlers::Writer;
use super::message::{Message, MessageData};
use super::cycle::Cycle;
use super::instrument::Instrument;

pub struct Grid {
    width: u8,
    height: u8,
    base_note: u8,
    active_leds: Vec<u8>,
}

// TODO - undraw & redraw?
impl Grid {
    pub fn new(width: u8, height: u8, base_note: u8) -> Self {
        Grid { width, height, base_note, active_leds: vec![] }
    }

    fn draw_led(channel: u8, note: u8, state: u8, frame: u32, writer: &mut Writer) {
        writer.write(Message::new(frame, MessageData::Note([channel, note, state])));
    }

    pub fn save_led_state(&mut self, led: u8, state: u8) {
        if state > 0 {
            if ! self.active_leds.contains(&led) {
                self.active_leds.push(led);
            }
        } else {
            self.active_leds.retain(|active_led| {
                &led != active_led
            })
        }
    }

    // Do not allow switching leds outside of grid
    pub fn try_switch_led(&mut self, x: i32, y: i32, state: u8, frame: u32, writer: &mut Writer) {
        if x >= self.width as i32 || x < 0 || y >= self.height as i32 || y < 0 {
            ()
        } else {
            self.switch_led(x as u8, y as u8, state, frame, writer);
        }
    }

    pub fn switch_led(&mut self, x: u8, y: u8, state: u8, frame: u32, writer: &mut Writer) {
        self.save_led_state(y * self.width + x, state);

        Grid::draw_led(0x90 + x, self.base_note + y, state, frame, writer);
    }

    pub fn clear_active(&mut self, frame: u32, writer: &mut Writer) {
        self.active_leds.iter()
            .for_each(|led| {
                Grid::draw_led(0x90 + led % self.width, self.base_note + led / self.width, 0, frame, writer);
            });

        self.active_leds.clear();
    }

    pub fn clear(&mut self, writer: &mut Writer) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.switch_led(x, y, 0, 0, writer);
            }
        }
    }
}

/*
impl Grid {
    fn new() -> Self {
        Grid {
            // Vector containing current state of leds in our sequencer grid
            active_leds: vec![],
            // A4 should be at the bottom, grid is 5 leds high
            base_note: 69 + 4,
        }
    }
    
    // TODO - Make trait

    fn clear(&mut self, writer: &mut Writer) {
        // Active pattern 1
        self.switch_led(0, 0x52, 0, writer);
        // Inactive pattern 2
        self.switch_led(0, 0x53, 0, writer);
        self.switch_led(0, 0x54, 0, writer);

        // Active track
        self.switch_led(0, 0x33, 0, writer);
        // Alternate tracks active
        self.switch_led(0, 0x50, 0, writer);

        // Velocity
        (0..8).for_each(|led| {
            self.switch_led(led, 0x30, 0, writer);
        });

        // Clear length indicator
        (0..8).for_each(|led| {
            self.switch_led(led, 0x32, 0, writer);
        });
        // Clear grid
        (0..40).for_each(|led| {
            self.switch_led(led % 8, 0x35 + led / 8, 0, writer);
        });
    }
}
*/

pub struct Sequencer {
    instruments: Vec<Instrument>,
    active_instrument: usize,
    // Keep track of elapsed ticks to trigger note_off when transport stops
    was_repositioned: bool,
}

impl Sequencer {
    pub fn new() -> Self {
        let mut instruments = vec![Instrument::default(0)];
        instruments.append(&mut (1..16).map(|channel| { Instrument::new(channel) }).collect());

        Sequencer{
            instruments,
            active_instrument: 0,
            was_repositioned: true,
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
        //self.pattern.output_note_off_events(&cycle, midi_out);

        let active_instrument = &mut self.instruments[self.active_instrument];

        active_instrument.draw(cycle, self.was_repositioned, control_out);

        self.instruments.iter_mut().for_each(|instrument| {
            instrument.output(cycle, midi_out);
        });

        self.update(cycle);
    }
}
