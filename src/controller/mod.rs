
pub mod input;
mod lights;

use std::ops::Range;
use super::TickRange;
use super::message::{TimedMessage, Message};
use super::cycle::ProcessCycle;
use super::loopable::*;
use super::sequencer::*;
use super::surface::*;
use super::port::MidiOut;
use super::mixer::*;
use super::TimebaseHandler;
use super::display::*;
use super::memory::*;
use super::track::*;
use super::events::*;
use input::*;
use lights::*;

const SEQUENCE_COLOR: u8 = 1;
const TIMELINE_HEAD_COLOR: u8 = 1;
const TIMELINE_TAIL_COLOR: u8 = 3;
// Same as default phrase length atm
// Wait some cycles for sloooow apc's
const IDENTIFY_CYCLES: u8 = 3;
const LENGTH_INDICATOR_USECS: u64 = 200000;
const DOUBLE_CLICK_USECS: u64 = 300000;
const PLAYING_LOOPABLE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32;
const PLAYING_SEQUENCE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32;
const QUEUED_SEQUENCE_INDICATOR_TICKS: u32 = TimebaseHandler::TICKS_PER_BEAT as u32 / 2;

pub struct APC {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    pub button_offset_x: u8,
    pub button_offset_y: u8,
    controller_input: ControllerInput,

    identified_cycles: u8,
    pub is_identified: bool,
    //knob_offset: u8,

    master: Single,
    grid: Grid,
    side: Side,
    indicator: WideRow,
    track: WideRow,
    activator: WideRow,
    solo: WideRow,
    //arm: WideRow,
}

impl APC {
    pub fn new(client: &jack::Client, name: &str, button_offset_x: u8, button_offset_y: u8, controller_input: ControllerInput) -> Self {
        let input = client.register_port(&(name.to_owned() + " in"), jack::MidiIn::default()).unwrap();
        let output = client.register_port(&(name.to_owned() + " out"), jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            button_offset_x,
            button_offset_y,
            controller_input,

            identified_cycles: 0,
            is_identified: false,
            // Offset knobs by this value to support multiple groups
            //knob_offset: 0,

            master: Single::new(0x50),
            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            track: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            // TODO - Put length indicator here, get length from longest LoopablePatternEvent in phrases?
            //arm: WideRow::new(0x30),
        }
    }

    /*
     * Get input events from raw midi events
     */
    pub fn input_events(&self, cycle: &ProcessCycle) -> Vec<InputEvent> {
        self.input.iter(cycle.scope)
            .map(|message| self.controller_input.message_to_input_event(message, self.button_offset_x, self.button_offset_y))
            .collect()
    }

    /*
     * Try to identify this controller
     */
    pub fn identify(&mut self, cycle: &ProcessCycle) {
        for event in self.input_events(cycle).iter() {
            // Process global events
            match event.event_type {
                InputEventType::InquiryResponse(local_id, device_id) => {
                    // Make sure we stop inquiring
                    // TODO - Make sure every grid is re-initialized after identifying
                    self.identified_cycles = 1;

                    // Introduce ourselves to controller
                    // 0x41 after 0x04 is ableton mode (only led rings are not controlled by host, but can be set.)
                    // 0x42 is ableton alternate mode (all leds controlled from host)
                    let message = Message::Introduction([0xF0, 0x47, local_id, device_id, 0x60, 0x00, 0x04, 0x41, 0x00, 0x00, 0x00, 0xF7]);
                    self.output.write_message(&cycle.scope, TimedMessage::new(0, message));
                },
                _ => (),
            }
        }

        if self.identified_cycles == 0 {
            self.output.write_message(&cycle.scope, TimedMessage::new(0, Message::Inquiry([0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7])));
        } else if self.identified_cycles < IDENTIFY_CYCLES {
            // We have to wait some cycles as controller takes some time to initialize
            self.identified_cycles = self.identified_cycles + 1;
        } else {
            self.is_identified = true;
        }
    }

    /*
     * Draw loopable display
     */
    pub fn draw_display(&mut self, display: &impl Display, button_offset_x: u8, grid_width: u8, memory: &Memory, track: &Track, track_index: usize) {
        let grid_stop = display.parameters().offset_x() + display.parameters().ticks_in_grid(grid_width);
        let ticks_per_button = display.parameters().ticks_per_button() as i32;
        let loopable = display.loopable(track, track_index);

        // Draw main grid
        loopable.events().iter()
            .filter(|event| { 
                let grid_contains_event = event.start() < grid_stop 
                    && (event.stop().is_none() || event.stop().unwrap() > display.parameters().offset_x());

                grid_contains_event || event.is_looping()
            })
            .for_each(|event| {
                // Get buttons from event ticks
                let max_button = self.grid.width() as i32;
                let start_button = (event.start() as i32 - display.parameters().offset_x() as i32) / ticks_per_button;
                let stop_button = if event.stop().is_none() { 
                    start_button + 1
                } else { 
                    // Could be event is to short for 1 button, in that case, draw 1 button
                    // TODO
                    (event.stop().unwrap() as i32 - display.parameters().offset_x() as i32) / ticks_per_button
                };

                // Flip grid around to show higher notes higher on the grid (for patterns this does not matter)
                let row = event.row(display.parameters().offset_y());

                // Always draw first button head
                self.grid.try_draw(start_button, row, Self::ledcolor_to_int(&display.parameters().head_color()));

                // Draw tail depending on wether this is looping note
                let tails = if stop_button >= start_button {
                    vec![(start_button + 1) .. stop_button]
                } else {
                    vec![(start_button + 1) .. max_button, 0 .. stop_button]
                };

                tails.into_iter().for_each(|mut x_range| {
                    for x in x_range {
                        self.grid.try_draw(x, row, Self::ledcolor_to_int(&display.parameters().tail_color())) 
                    }
                })
            });

        // pattern length selector
        if loopable.has_explicit_length() {
            for index in 0 .. loopable.length_factor() {
                self.activator.draw(index as u8, 1);
            }
        }
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        let mut messages = vec![];

        messages.append(&mut self.grid.output_messages(0));
        messages.append(&mut self.activator.output_messages(0));

        // from this function
        self.output.write_messages(cycle.scope, &mut messages);
    }

    fn ledcolor_to_int(color: &LedColor) -> u8 {
        match color {
            LedColor::Red => 3,
            LedColor::Orange => 5,
            LedColor::Green => 1,
        }
    }
}

