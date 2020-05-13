
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

    button_offset_x: u8,
    button_offset_y: u8,
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
}

