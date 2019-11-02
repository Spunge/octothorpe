
use std::ops::Range;
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
use super::instrument::Instrument;
use super::handlers::TimebaseHandler;
use super::phrase::PlayingPhrase;
use super::pattern::PlayingPattern;
use super::sequence::Sequence;
use super::playable::Playable;
use super::note::Note;

pub enum OverView {
    Instrument,
    Sequence,
}

pub enum DetailView {
    Pattern,
    Phrase,
}

#[derive(Debug, Clone)]
pub struct KeyPress {
    channel: u8,
    note: u8,
    velocity: u8,
}

impl KeyPress {
    fn new(message: jack::RawMidi) -> Self {
        KeyPress {
            channel: message.bytes[0],
            note: message.bytes[1],
            velocity: message.bytes[2],
        }
    }
}

pub struct Sequencer {
    sequence_note_offs: Vec<(u32, Message)>,
    indicator_note_offs: Vec<(u32, Message)>,
    keys_pressed: Vec<KeyPress>,

    instruments: [Instrument; 16],
    pub instrument_group: u8,
    instrument: u8,

    pub keyboard_target: u8,
    pub drumpad_target: u8,
    is_quantizing: bool,

    sequences: [Sequence; 4],
    sequence: u8,

    // What is playing?
    sequence_playing: usize,
    sequence_queued: Option<usize>,

    // What are we showing?
    overview: OverView,
    detailview: DetailView,

    pub should_render: bool,
    state_current: [u8; 96],
    state_next: [u8; 96],
    // Buttons
    index_instrument_group: Range<usize>,
    index_detailview: Range<usize>,
    index_quantizing: Range<usize>,
    // Static
    index_knob_groups: Range<usize>,
    index_instruments: Range<usize>,
    index_main: Range<usize>,
    index_green: Range<usize>,
    index_blue: Range<usize>,
    index_red: Range<usize>,

    index_indicator: Range<usize>,
    index_playables: Range<usize>,
    index_sequences: Range<usize>,
}

impl Sequencer {
    pub fn new() -> Self {
        // Build instruments for each midi channel
        let instruments = [
            Instrument::new(0), Instrument::new(1), Instrument::new(2), Instrument::new(3),
            Instrument::new(4), Instrument::new(5), Instrument::new(6), Instrument::new(7),
            Instrument::new(8), Instrument::new(9), Instrument::new(10), Instrument::new(11),
            Instrument::new(12), Instrument::new(13), Instrument::new(14), Instrument::new(15),
        ];
    
        // Build sequence we can trigger
        let sequences = [ Sequence::default(0), Sequence::default(1), Sequence::default(2), Sequence::default(3), ];

        Sequencer {
            instruments,
            instrument_group: 0,
            instrument: 0,

            keyboard_target: 0,
            drumpad_target: 0,
            is_quantizing: true,

            keys_pressed: vec![],
            sequence_note_offs: vec![],
            indicator_note_offs: vec![],

            sequences,
            sequence: 0,

            sequence_playing: 0,
            sequence_queued: Some(0),

            // What are we currently showing?
            detailview: DetailView::Pattern,
            overview: OverView::Instrument,

            // Static button states
            should_render: false,
            state_current: [0; 96],
            state_next: [0; 96],
            // Indexes of where in the state array grid state is located
            index_main: 0..40,
            index_green: 40..48,
            index_instruments: 48..56,
            index_red: 56..64,
            index_blue: 64..72,
            // Static buttons
            index_instrument_group: 72..73,
            index_detailview: 73..74,
            index_quantizing: 74..75,
            index_knob_groups: 75..79,
            // Dynamic button states
            index_indicator: 79..87,
            index_playables: 87..92,
            index_sequences: 92..96,
        }
    }

    fn instrument_index(&self) -> usize {
        (self.instrument_group * 8 + self.instrument) as usize
    }

    fn instrument(&mut self) -> &mut Instrument {
        &mut self.instruments[self.instrument_index()]
    }

    fn sequence(&mut self) -> &mut Sequence {
        &mut self.sequences[self.sequence as usize]
    }

    fn is_shift_pressed(&self) -> bool {
        self.keys_pressed.iter().any(|keypress| {
            keypress.note == 0x62 && keypress.velocity == 0x7F && keypress.channel == 0x90
        })
    }

    fn instrument_key_pressed(&mut self, message: jack::RawMidi) {
        match message.bytes[1] {
            0x3E | 0x51 => self.switch_detailview(),
            // Playable grid
            0x52 ..= 0x56 => {
                // Get start & end in grid of pressed keys
                let from = self.keys_pressed.iter()
                    .find(|keypress| [0x52, 0x53, 0x54, 0x55, 0x56].contains(&keypress.note))
                    .unwrap()
                    .note;

                let to = self.keys_pressed.iter().rev()
                    .find(|keypress| [0x52, 0x53, 0x54, 0x55, 0x56].contains(&keypress.note))
                    .unwrap()
                    .note;

                if from != to {
                    self.copy_playable(from - 0x52, to - 0x52);
                }

                self.switch_playable(to - 0x52);
            },
            // Grid should add notes & add phrases
            0x35 ..= 0x39 => {
                // Get start & end in grid of pressed keys
                let from = self.keys_pressed.iter()
                    .filter(|keypress| keypress.note == message.bytes[1])
                    .min_by_key(|keypress| keypress.channel)
                    .unwrap()
                    .channel - 0x90;

                let to = self.keys_pressed.iter()
                    .filter(|keypress| keypress.note == message.bytes[1])
                    .max_by_key(|keypress| keypress.channel)
                    .unwrap()
                    .channel - 0x90;

                match self.detailview {
                    DetailView::Pattern => self.instrument().pattern().toggle_led_range(from..to, message.bytes[1] - 0x35, 127, 127),
                    DetailView::Phrase => self.instrument().phrase().toggle_pattern(from..to, message.bytes[1] - 0x35),
                }
            },
            0x5E => self.instrument().pattern().change_base_note(4),
            0x5F => self.instrument().pattern().change_base_note(-4),
            0x30 => self.instrument().change_quantize_level((message.bytes[0] - 0x90 + 1) as u8),
            0x31 => self.playable().change_zoom((message.bytes[0] - 0x90 + 1) as u32),
            0x32 => {
                match self.detailview {
                    DetailView::Pattern => self.instrument().pattern().change_length((message.bytes[0] - 0x90 + 1) as u32),
                    DetailView::Phrase => self.instrument().phrase().change_length((message.bytes[0] - 0x90 + 1) as u32),
                }
            },
            0x61 => self.playable().change_offset(-1),
            0x60 => self.playable().change_offset(1),
            _ => (),
        }
    }

    fn sequence_key_pressed(&mut self, message: jack::RawMidi) {
        let instrument_delta = self.instrument_group * 8;

        match message.bytes[1] {
            0x32 => self.sequence().toggle_active(message.bytes[0] - 0x90 + instrument_delta),
            0x35 ..= 0x39 => {
                let instrument = message.bytes[0] - 0x90 + instrument_delta;
                self.sequence().toggle_phrase(instrument, message.bytes[1] - 0x35);
            },
            0x52 ..= 0x56 => self.sequence().toggle_row(message.bytes[1] - 0x52),
            _ => (),
        }
    }

    pub fn shared_key_pressed(&mut self, message: jack::RawMidi) {
        match self.overview {
            OverView::Instrument => self.instrument_key_pressed(message),
            OverView::Sequence => self.sequence_key_pressed(message),
        }
    }

    pub fn key_pressed(&mut self, message: jack::RawMidi) {
        //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);

        // Remember remember
        self.keys_pressed.push(KeyPress::new(message));

        match message.bytes[1] {
            // TODO - On switching instrument_group && instrument etc, draw indicator with cycle
            0x50 => self.switch_instrument_group(),
            0x3A ..= 0x3D => self.switch_knob_group(message.bytes[1] - 0x3A),
            0x33 => self.switch_instrument(message.bytes[0] - 0x90),
            0x3F => self.switch_quantizing(),
            0x57 ..= 0x5A => self.switch_sequence(message.bytes[1] - 0x57),
            0x3E | 0x30 | 0x31 | 0x32 | 0x60 | 0x61 | 0x51 | 0x5E | 0x5F => self.shared_key_pressed(message),
            // Playable select
            0x52 ..= 0x56 => self.shared_key_pressed(message),
            // Main grid
            0x35 ..= 0x39 => self.shared_key_pressed(message),
            _ => (),
        };

        self.should_render = true;
    }

    pub fn key_double_pressed(&mut self, message: jack::RawMidi) -> Option<Vec<TimedMessage>> {
        match message.bytes[1] {
            // Playable grid
            0x52 ..= 0x56 => {
                match self.overview {
                    OverView::Instrument => self.record_playable(message.bytes[1] - 0x52),
                    _ => (),
                }
            },
            _ => (),
        };

        None
    }

    // Key released is 0x80 + channel instead of 0x90 + channel
    pub fn key_released(&mut self, message: jack::RawMidi) -> Option<Vec<TimedMessage>> {
        self.keys_pressed.retain(|key_pressed| {
            key_pressed.channel != message.bytes[0] + 16
                || key_pressed.note != message.bytes[1]
                || key_pressed.velocity != message.bytes[2]
        });

        None
    }

    // One of the control knobs on the APC was turned
    fn knob_turned(&mut self, time: u32, knob: u8, value: u8) -> Vec<TimedMessage> {
        // Get the channel & knob that APC knob should send out of, first 8 channels are for
        // instruments, next 2 are used for sequences (sequence channels will be used for bus
        // effects)
        let (out_channel, out_knob) = match self.overview {
            OverView::Instrument => {
                let instrument_knob = self.instrument().set_knob_value(knob, value);
                let instrument = self.instrument_index() as u8;
                let offset = (instrument % 2) * 64;
                // 2 instruments per channel
                (instrument / 2, instrument_knob + offset)
            },
            OverView::Sequence => {
                let sequence_knob = self.sequence().set_knob_value(knob, value);
                // Sequence channels are after the instruments channels, therefore +
                // instruments.len / 2
                (self.sequence / 2 + self.instruments.len() as u8 / 2, sequence_knob + (self.sequence % 2) * 64)
            },
        };

        //println!("ME: knob_{:?} on channel {:?} turned to value: {:?}", out_knob, out_channel, value);
        vec![TimedMessage::new(time, Message::Note([0xB0 + out_channel, out_knob, value]))]
    }

    // Cue knob
    pub fn cue_knob_turned(&mut self, value: u8) {
        if let OverView::Instrument = self.overview {
            if let DetailView::Pattern = self.detailview {
                let delta = if value >= 64 { value as i32 - 128 } else { value as i32 };

                self.instrument().pattern().change_base_note(delta);
            }
        }

        self.should_render = true;
    }

    fn fader_adjusted(&mut self, time: u32, fader: u8, value: u8) -> Vec<TimedMessage> {
        // Output on channel 16
        let out_knob = fader + self.instrument_group * 8;
        vec![TimedMessage::new(time, Message::Note([0xB0 + 15, out_knob, value]))]
    }

    fn master_adjusted(&mut self, time: u32, value: u8) -> Vec<TimedMessage> {
        vec![TimedMessage::new(time, Message::Note([0xB0 + 15, 127, value]))]
    }

    pub fn control_changed(&mut self, message: jack::RawMidi) -> Option<Vec<TimedMessage>> {
        // APC knobs are ordered weird, reorder them from to 0..16
        match message.bytes[1] {
            0x10..=0x17 => Some(self.knob_turned(message.time, message.bytes[1] - 8, message.bytes[2])),
            0x30..=0x37 => Some(self.knob_turned(message.time, message.bytes[1] - 48, message.bytes[2])),
            0x7 => Some(self.fader_adjusted(message.time, message.bytes[0] - 0xB0, message.bytes[2])),
            0xE => Some(self.master_adjusted(message.time, message.bytes[2])),
            _ => None,
        }
    }

    pub fn plugin_parameter_changed(&mut self, message: jack::RawMidi) -> Option<TimedMessage> {
        //println!("SYNTHPOD: knob_{:?} on channel {:?} turned to value: {:?}", message.bytes[1], message.bytes[0] - 0xB0, message.bytes[2]);

        let mut knob = message.bytes[1];
        // Collections of 64 knobs
        let mut knob_collection = (message.bytes[0] - 0xB0) * 2;

        // Mulitple sequences and instruments live in same channels
        if knob >= 64 {
            knob_collection = knob_collection + 1;
            knob = knob - 64;
        }

        let knob_group = knob / 16;
        let apc_knob = knob % 16;

        // Pass change to correct knob group container
        let changed_knob = if knob_collection < 16 {
            self.instruments[knob_collection as usize].knob_value_changed(knob, message.bytes[2])
                // Knob was changed, see if instrument & knob_group are currently shown, if it is,
                // update knob on APC
                .and_then(|_| {
                    // Check if changed virtual knob is visible at the moment
                    if let OverView::Instrument = self.overview {
                        if self.instrument_index() as u8 == knob_collection && self.instrument().knob_group == knob_group {
                            Some(apc_knob)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
        } else {
            // TODO - Sequence knob groups broken!
            self.sequences[knob_collection as usize - 16].knob_value_changed(knob, message.bytes[2])
                .and_then(|_| {
                    if let OverView::Sequence = self.overview {
                        // Sequence knob collections are placed after instrument groups
                        let sequence = (knob_collection - 16) / 4;
                        if self.sequence == sequence && self.sequence().knob_group == knob_group {
                            Some(apc_knob)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
        };

        // If knob was actually changed, pass message through to APC
        changed_knob
            .and_then(|mut knob| {
                knob = if knob < 8 { 0x30 + knob } else { 0x10 + knob - 8 };
                Some(TimedMessage::new(message.time, Message::Note([0xB0, knob, message.bytes[2]])))
            })
    }

    pub fn recording_key_played(&mut self, instrument: u8, raw_channel: u8, cycle: &Cycle, message: jack::RawMidi) -> TimedMessage {
        // We're subtracting 9 as drumpads of my keyboard are outputting on channel 9
        let mut instrument_channel = raw_channel + instrument;

        if cycle.is_rolling {
            // Could be this is a note down meant for previously selected instrument
            let target: &mut Instrument = if self.instruments[instrument as usize].recorded_messages.len() == 0 && raw_channel == 0x80 {
                // Send message to instrument that has recorded messages left
                let (index, instrument) = self.instruments.iter_mut().enumerate()
                    .find(|(_, instrument)| instrument.recorded_messages.len() > 0)
                    .unwrap();

                instrument_channel = raw_channel + index as u8;
                instrument
            } else {
                // It was meant for current instrument instead
                &mut self.instruments[instrument as usize]
            };

            // Only record when cycle is rolling
            let cycle_length = cycle.end - cycle.start;
            // Message was recorded in previous frame
            let message_time = (cycle.start - cycle_length) + message.time;
            target.record_message(message_time, raw_channel, message.bytes[1], message.bytes[2], self.is_quantizing);
        }

        // Always play the note
        TimedMessage::new(message.time, Message::Note([instrument_channel, message.bytes[1], message.bytes[2]])) 
    }

    fn switch_instrument_group(&mut self) {
        self.instrument_group = if self.instrument_group == 1 { 0 } else { 1 };
    }

    fn switch_knob_group(&mut self, group: u8) {
        match self.overview {
            OverView::Instrument => self.instrument().switch_knob_group(group),
            OverView::Sequence => self.sequence().switch_knob_group(group),
        }
    }

    fn switch_sequence(&mut self, sequence: u8) {
        // Queue sequence
        if self.is_shift_pressed() {
            self.sequence_queued = Some(sequence as usize);
        } else {
            // When we press currently selected overview, return to instrument view, so we can peek
            if self.sequence == sequence {
                self.switch_overview();
            } else {
                // If we select a new sequence, show that
                self.sequence = sequence;

                if let OverView::Instrument = self.overview {
                    self.switch_overview();
                }
            }
        }
    }

    fn switch_instrument(&mut self, instrument: u8) {
        // If we click selected instrument, return to sequence for peeking
        if self.instrument == instrument {
            self.switch_overview();
        } else {
            // Otherwise select instrument && switch
            self.instrument = instrument;
            self.keyboard_target = instrument;
            self.drumpad_target = instrument;

            if let OverView::Sequence = self.overview {
                self.switch_overview();
            }
        }
    }

    fn switch_quantizing(&mut self) {
        self.is_quantizing = ! self.is_quantizing;
    }

    // Set a playable to recording mode
    fn record_playable(&mut self, playable: u8) {
        match self.detailview {
            DetailView::Pattern => {
                self.instrument().patterns[playable as usize].switch_recording_state()
            },
            _ => (),
        }
    }

    fn copy_playable(&mut self, from: u8, to: u8) {
        match self.detailview {
            DetailView::Pattern => {
                self.instrument().patterns[to as usize] = self.instrument().patterns[from as usize].clone();
            },
            DetailView::Phrase => {
                self.instrument().phrases[to as usize] = self.instrument().phrases[from as usize].clone();
            },
        }
    }

    fn switch_playable(&mut self, playable: u8) {
        match self.detailview {
            DetailView::Pattern => self.instrument().pattern = playable as usize,
            DetailView::Phrase => self.instrument().phrase = playable as usize,
        }

        // Reset pattern on shift click
        if self.is_shift_pressed() {
            match self.detailview {
                DetailView::Pattern => self.instrument().pattern().reset(),
                _ => (),
            }
        }
    }

    fn switch_overview(&mut self) {
        match self.overview {
            OverView::Instrument => {
                self.overview = OverView::Sequence
            },
            OverView::Sequence => {
                // Clear as we do not want the selected instrument grid to clear
                self.indicator_note_offs = vec![];
                self.overview = OverView::Instrument
            },
        }
    }

    fn switch_detailview(&mut self) {
        match self.detailview {
            DetailView::Pattern => self.detailview = DetailView::Phrase,
            DetailView::Phrase => self.detailview = DetailView::Pattern,
        }
    }

    fn playable(&mut self) -> &mut Playable {
        match self.detailview {
            DetailView::Pattern => &mut self.instrument().pattern().playable,
            DetailView::Phrase => &mut self.instrument().phrase().playable,
        }
    }

    pub fn reset(&mut self) {
        // Use non-existant state to always redraw
        self.state_current = [9; 96];
        self.state_next = [0; 96];
        self.should_render = true;
    }

    pub fn output_vertical_grid(&mut self, range: Range<usize>, y: u8) -> Vec<Message> {
        let mut output = vec![];

        for index in range.start..range.end {
            if self.state_current[index] != self.state_next[index] {
                let x: u8 = if self.state_next[index] == 0 { 0x80 } else { 0x90 };

                output.push(Message::Note([x, y + (index - range.start) as u8, self.state_next[index]]));
            }
        }

        self.switch_state(range.clone());

        output
    }

    // Output a message for each changed state in the grid
    pub fn output_horizontal_grid(&mut self, range: Range<usize>, y: u8) -> Vec<Message> {
        let mut output = vec![];

        for index in range.start..range.end {
            if self.state_current[index] != self.state_next[index] {
                let x = if self.state_next[index] == 0 { 0x80 } else { 0x90 };

                output.push(Message::Note([x + (index - range.start) as u8 % 8, y + (index - range.start) as u8 / 8, self.state_next[index]]));
            }
        }

        self.switch_state(range.clone());

        output
    }

    pub fn output_button(&self, index: usize, y: u8) -> Option<Message> {
        if self.state_current[index] != self.state_next[index] {
            let x = if self.state_next[index] == 0 { 0x80 } else { 0x90 };
            Some(Message::Note([x, y, self.state_next[index]]))
        } else {
            None
        }
    }

    fn draw_main_grid(&mut self) {
        // Why do i have to do this?
        let instrument_group = self.instrument_group;

        let states = match self.overview {
            OverView::Instrument => match self.detailview {
                DetailView::Pattern => self.instrument().pattern().led_states(),
                DetailView::Phrase => self.instrument().phrase().led_states(),
            }
            OverView::Sequence => self.sequence().led_states(instrument_group),
        };

        // Get states that are within grid
        let valid_states = states.into_iter().filter(|(x, y, _)| {
            x < &8 && x >= &0 && y < &5 && y >= &0
        });

        for (x, y, state) in valid_states {
            self.state_next[y as usize * 8 + x as usize] = state;
        }
    }

    fn draw_red_grid(&mut self) {
        let quantize_level = match self.overview {
            OverView::Instrument => self.instrument().quantize_level as usize,
            _ => 0,
        };

        for index in self.index_red.clone() {
            let led = index - self.index_red.start;

            self.state_next[index] = if led < quantize_level { 1 } else { 0 };
        }
    }

    fn draw_green_grid(&mut self) {
        for index in self.index_green.clone() {
            let led = index - self.index_green.start;

            self.state_next[index] = match self.overview {
                // In instrument, green grid shows length of playable
                OverView::Instrument => {
                    let length = (self.playable().length / self.playable().minimum_length) as usize;
                    if led < length { 1 } else { 0 }
                },
                // In Sequence, green grid shows active instruments
                OverView::Sequence => {
                    let instrument = self.instrument_group as usize * 8 + led;
                    if self.sequence().active[instrument] { 1 } else { 0 }
                }
            }
        }
    }

    fn draw_blue_grid(&mut self) {
        let length = match self.overview {
            OverView::Instrument => 8 / self.playable().zoom as usize,
            _ => 0,
        };
        let start = self.playable().offset as usize * length;
        let end = start + length;

        for index in self.index_blue.clone() {
            let led = index - self.index_blue.start;

            self.state_next[index] = if led >= start && led < end { 1 } else { 0 };
        }
    }

    fn draw_instruments_grid(&mut self) {
        for index in self.index_instruments.clone() {
            let led = index - self.index_instruments.start;

            // Force clear as sequence indicator uses the same grid and does not clear it
            self.state_current[index] = 9;
            self.state_next[index] = match self.overview {
                OverView::Instrument => if led as u8 == self.instrument { 1 } else { 0 },
                _ => 0,
            };
        }
    }

    fn draw_knob_groups_grid(&mut self) {
        for index in self.index_knob_groups.clone() {
            let led = index - self.index_knob_groups.start;

            let active_group = match self.overview {
                OverView::Instrument => self.instrument().knob_group,
                OverView::Sequence => self.sequence().knob_group,
            };

            self.state_next[index] = if led as u8 == active_group { 1 } else { 0 };
        }
    }

    fn draw_group_button(&mut self) {
        self.state_next[self.index_instrument_group.start] = self.instrument_group;
    }

    fn draw_quantize_button(&mut self) {
        self.state_next[self.index_quantizing.start] = if self.is_quantizing { 1 } else { 0 };
    }

    fn draw_detailview_button(&mut self) {
        self.state_next[self.index_detailview.start] = match self.overview {
            OverView::Instrument => match self.detailview { DetailView::Pattern => 1, _ => 0 },
            _ => 0,
        };
    }

    pub fn switch_state(&mut self, range: Range<usize>) {
        for i in range.start..range.end {
            self.state_current[i] = self.state_next[i];
            self.state_next[i] = 0;
        }
    }

    pub fn force_clear_state(&mut self, range: Range<usize>) {
        for i in range.start..range.end {
            self.state_current[i] = 9;
        }
    }

    // Should render main grid is passed when there's notes played on sequence_in this frame
    pub fn output_static(&mut self, should_render_main_grid: bool) -> Vec<TimedMessage> {
        // Output vector
        let mut output = vec![];

        // Also render main grid when notes where input
        if self.should_render || should_render_main_grid {
            self.draw_main_grid();
            output.extend(self.output_horizontal_grid(self.index_main.clone(), 0x35));
        }

        // Draw if we have to
        if self.should_render {
            self.draw_red_grid();
            self.draw_green_grid();
            self.draw_blue_grid();
            self.draw_instruments_grid();
            self.draw_knob_groups_grid();
            self.draw_group_button();
            self.draw_quantize_button();
            self.draw_detailview_button();

            output.extend(self.output_horizontal_grid(self.index_red.clone(), 0x30));
            output.extend(self.output_horizontal_grid(self.index_green.clone(), 0x32));
            output.extend(self.output_horizontal_grid(self.index_blue.clone(), 0x31));
            output.extend(self.output_horizontal_grid(self.index_instruments.clone(), 0x33));
            output.extend(self.output_vertical_grid(self.index_knob_groups.clone(), 0x3A));
            output.extend(self.output_horizontal_grid(self.index_instrument_group.clone(), 0x50));
            output.extend(self.output_horizontal_grid(self.index_quantizing.clone(), 0x3F));
            output.extend(self.output_horizontal_grid(self.index_detailview.clone(), 0x3E));

            // Clear dynamic grids when switching to sequence
            if let OverView::Sequence = self.overview {
                self.force_clear_state(self.index_indicator.clone());
                //output.extend(self.output_horizontal_grid(self.index_indicator.clone(), 0x34));
                self.force_clear_state(self.index_playables.clone());
                output.extend(self.output_vertical_grid(self.index_playables.clone(), 0x52));
            }

            // Output knob values for currently selected inst/seq
            let knob_values = match self.overview {
                OverView::Instrument => self.instrument().get_knob_values(),
                OverView::Sequence => self.sequence().get_knob_values(),
            };

            for (index, value) in knob_values.iter().enumerate() {
                // Get APC knob id from instrumen knob id
                let knob_id = if index < 8 { 0x30 } else { 0x10 - 8 } + index;
                //output.extend(Message::)
                // 0xB0 = apc control channel
                output.push(Message::Note([0xB0, knob_id as u8, *value]));
            }

            self.should_render = false;
        }

        output.into_iter().map(|message| TimedMessage::new(0, message)).collect()
    }

    // Get playing phrases that fall in this cycle
    fn playing_phrases(&mut self, cycle: &Cycle) -> Option<Vec<PlayingPhrase>> {
        let playing_sequence = &self.sequences[self.sequence_playing];

        if let Some(sequence_length) = playing_sequence.length(&self.instruments) {
            // A sequence with length above 0 is playing
            let sequence_start = (cycle.start / sequence_length) * sequence_length;
            let sequence_end = sequence_start + sequence_length;

            let mut phrases = playing_sequence.playing_phrases(&self.instruments, sequence_start);

            // Cycle is passing sequence, end, add next playing sequence
            if sequence_end < cycle.end {
                let next_sequence_start = sequence_start + sequence_length;

                if let Some(index) = self.sequence_queued {
                    phrases.extend(self.sequences[index].playing_phrases(&self.instruments, next_sequence_start))
                } else {
                    phrases.extend(playing_sequence.playing_phrases(&self.instruments, next_sequence_start))
                }
            }

            // Activate next sequence
            if sequence_end <= cycle.end {
                if let Some(index) = self.sequence_queued {
                    // Mark sequence as switched
                    self.sequence_playing = index;
                    self.sequence_queued = None;
                }
            }

            Some(phrases)
        } else {
            // 0 length sequence, so nothing is playing
            // TODO - Check quueued here?
            None
        }
    }

    fn playing_patterns(&self, cycle: &Cycle, playing_phrases: &Vec<PlayingPhrase>) -> Vec<PlayingPattern> {
        playing_phrases.iter()
            // Get patterns that are playing for Instrument & played pattern
            .flat_map(|playing_phrases| {
                self.instruments[playing_phrases.instrument].phrases[playing_phrases.phrase]
                    .playing_patterns(&self.instruments[playing_phrases.instrument].patterns, &playing_phrases)
            })
            .filter(|playing_pattern| playing_pattern.start < cycle.end && playing_pattern.end > cycle.start)
            .collect()
    }

    // Get notes that should be triggered in currently playing sequences
    fn playing_notes(&self, cycle: &Cycle, playing_patterns: &Vec<PlayingPattern>) -> Vec<(u32, &Note)> {
        // Get phrases that are playing in sequence
        // ( instrument, phrase )
        playing_patterns.into_iter()
            // Next, get notes for each instrument / played pattern
            .flat_map(|playing_pattern| {
                self.instruments[playing_pattern.instrument].patterns[playing_pattern.pattern]
                    .playing_notes(cycle, playing_pattern.start, playing_pattern.end)
            })
            .collect()
    }

    // Get messages for noteoffs that fall in this frame
    fn note_off_messages(cycle: &Cycle, buffer: &mut Vec<(u32, Message)>) -> Vec<TimedMessage> {
        let mut messages = vec![];

        // Get noteoffs that occur in this cycle
        for index in (0..buffer.len()).rev() {
            let (absolute_tick, _) = buffer[index];

            if let Some(frame) = cycle.delta_frames_absolute(absolute_tick) {
                let (_, message) = buffer.swap_remove(index);
                messages.push(TimedMessage::new(frame, message));
            }
        }

        messages
    }

    fn sequence_note_events(cycle: &Cycle, notes: &Vec<(u32, &Note)>, modifier: u32, channel_modifier: u8, key: Option<u8>, velocity_on: Option<u8>, velocity_off: Option<u8>) 
        -> (Vec<(u32, Message)>, Vec<TimedMessage>) 
    {
        let note_offs: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let length = note.end - note.start;
                let tick = cycle.absolute_start + delta_ticks;

                (tick + length / modifier, note.off_message(0x80 - channel_modifier, key, velocity_off))
            })
            .collect();

        let note_ons: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let delta_frames = (*delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;
                TimedMessage::new(delta_frames, note.on_message(0x90 - channel_modifier, key, velocity_on))
            })
            .collect();

        (note_offs, note_ons)
    }

    // Show playing, queued and selected sequence
    fn sequence_indicator_note_events(&mut self, cycle: &Cycle, force_redraw: bool) -> Option<Vec<TimedMessage>> {
        let playing_ticks = TimebaseHandler::beats_to_ticks(1.0);

        cycle.delta_ticks_recurring(0, playing_ticks)
            // Are we forced to redraw? If yes, instantly draw
            .or_else(|| if force_redraw { Some(0) } else { None })
            .and_then(|delta_ticks| {
                let switch_on_tick = cycle.start + delta_ticks;

                // Add queued when it's there
                if let Some(index) = self.sequence_queued {
                    self.state_next[self.index_sequences.start + index] = 1;
                }

                // Set playing sequence
                self.state_next[self.index_sequences.start + self.sequence_playing] = if ((switch_on_tick / playing_ticks) % 2) == 0 { 1 } else { 0 };

                // Create timed messages from indicator state
                let messages = self.output_vertical_grid(self.index_sequences.clone(), 0x57).into_iter()
                    .map(|message| TimedMessage::new(cycle.ticks_to_frames(delta_ticks), message))
                    .collect();

                // Return these beautifull messages
                Some(messages)
            })
    }
        
    // Show playing & selected pattern or phrase
    fn playable_indicator_note_events(&mut self, cycle: &Cycle, force_redraw: bool, playing_patterns: &Vec<PlayingPattern>, playing_phrases: &Vec<PlayingPhrase>) 
        -> Option<Vec<TimedMessage>> 
    {
        let playing_ticks = TimebaseHandler::beats_to_ticks(1.0);
        let recording_ticks = TimebaseHandler::beats_to_ticks(0.5);

        cycle.delta_ticks_recurring(0, recording_ticks)
            // Are we forced to redraw? If yes, instantly draw
            .or_else(|| if force_redraw { Some(0) } else { None })
            .and_then(|delta_ticks| {
                let switch_on_tick = cycle.start + delta_ticks;

                // Make playing playable blink
                let playing_indexes: Vec<usize> = match self.detailview {
                    DetailView::Pattern => {
                        playing_patterns.iter()
                            .filter(|playing_pattern| {
                                playing_pattern.instrument == self.instrument_index()
                                    && playing_pattern.end > cycle.end
                            })
                            .map(|playing_pattern| playing_pattern.pattern)
                            .collect()
                    }
                    DetailView::Phrase => {
                        playing_phrases.iter()
                            .filter(|playing_phrase| {
                                playing_phrase.instrument == self.instrument_index()
                                    && playing_phrase.end > cycle.end
                            })
                            .map(|playing_phrase| playing_phrase.phrase)
                            .collect()
                    },
                };

                // Multiple patterns or phrases can be playing
                playing_indexes.into_iter().for_each(|index| {
                    self.state_next[self.index_playables.start + index] = if ((switch_on_tick / playing_ticks) % 2) == 0 { 1 } else { 0 };
                });

                // Always mark selected playable
                let selected_index = match self.detailview {
                    DetailView::Pattern => self.instrument().pattern,
                    DetailView::Phrase => self.instrument().phrase,
                };
                self.state_next[self.index_playables.start + selected_index] = 1;

                // Always (most importantly, so last) render recording playables
                let recording_indexes: Vec<usize> = match self.detailview {
                    DetailView::Pattern => {
                        self.instrument().patterns.iter().enumerate()
                            .filter_map(|(index, pattern)| {
                                if pattern.is_recording { Some(index) } else { None }
                            })
                            .collect()
                    }
                    _ => vec![],
                };

                recording_indexes.into_iter().for_each(|index| {
                    self.state_next[self.index_playables.start + index] = if ((switch_on_tick / recording_ticks) % 2) == 0 { 1 } else { 0 };
                });

                // Create timed messages from indicator state
                let messages = self.output_vertical_grid(self.index_playables.clone(), 0x52).into_iter()
                    .map(|message| TimedMessage::new(cycle.ticks_to_frames(delta_ticks), message))
                    .collect();

                // Return these beautifull messages
                Some(messages)
            })
    }

    fn main_indicator_note_events(&mut self, cycle: &Cycle, force_redraw: bool, playing_patterns: &Vec<PlayingPattern>, playing_phrases: &Vec<PlayingPhrase>) 
        -> Option<Vec<TimedMessage>> 
    {
        // Minimum switch time for sequence indicator
        let mut ticks_interval = self.playable().ticks_per_led();
        // TODO - Ugly hotfix, plz fix
        if ticks_interval > TimebaseHandler::beats_to_ticks(1.0) {
            ticks_interval = TimebaseHandler::beats_to_ticks(1.0);
        }

        // Do we have to switch now?
        cycle.delta_ticks_recurring(0, ticks_interval)
            // Are we forced to redraw? If yes, instantly draw
            .or_else(|| if force_redraw { Some(0) } else { None })
            .and_then(|delta_ticks| {
                let switch_on_tick = cycle.start + delta_ticks;

                // Get playing regions of playables that are shown at the moment
                match self.overview {
                    OverView::Sequence => {
                        let longest_phrase = playing_phrases.into_iter()
                            .map(|playing_phrase| self.instruments[playing_phrase.instrument].phrases[playing_phrase.phrase].playable.length)
                            .max()
                            .unwrap();

                        let ticks_per_led = longest_phrase / 8;

                        let sequence_tick = switch_on_tick as i32 % longest_phrase as i32;
                        let led = sequence_tick / ticks_per_led as i32;

                        if led >= 0 && led < 8 && sequence_tick < longest_phrase as i32 {
                            self.state_next[self.index_indicator.start + led as usize] = 1;
                        }
                    },
                    OverView::Instrument => {
                        let shown_playables: Vec<(u32, u32)> = match self.detailview {
                            DetailView::Pattern => {
                                playing_patterns.into_iter()
                                    .filter(|playing_pattern| {
                                        playing_pattern.instrument == self.instrument_index()
                                            && playing_pattern.pattern == self.instruments[playing_pattern.instrument].pattern
                                    })
                                    .map(|playing_pattern| (playing_pattern.start, playing_pattern.end))
                                    .collect()
                            },
                            DetailView::Phrase => {
                                playing_phrases.into_iter()
                                    .filter(|playing_phrase| {
                                        playing_phrase.instrument == self.instrument_index()
                                            && playing_phrase.phrase == self.instruments[playing_phrase.instrument].phrase
                                    })
                                    .map(|playing_phrase| (playing_phrase.start, playing_phrase.end))
                                    .collect()
                            },
                        };

                        shown_playables.into_iter()
                            // Only show led for unfinished playables
                            .filter(|(_, end)| *end > cycle.end)
                            .for_each(|(start, end)| {
                                // Amount of ticks in region (playing patterns can be shorter as pattern they play)
                                let ticks = end - start;
                                let playable_tick = switch_on_tick as i32 - start as i32 - self.playable().ticks_offset() as i32;
                                let led = playable_tick / self.playable().ticks_per_led() as i32;

                                if led >= 0 && led < 8 && playable_tick < ticks as i32 {
                                    self.state_next[self.index_indicator.start + led as usize] = 1;
                                }
                            });
                    },
                };


                // Create timed messages from indicator state
                let messages = self.output_horizontal_grid(self.index_indicator.clone(), 0x34).into_iter()
                    .map(|message| TimedMessage::new(cycle.ticks_to_frames(delta_ticks), message))
                    .collect();

                // Return these beautifull messages
                Some(messages)
            })
    }

    // Send midi to process handler
    pub fn output_midi(&mut self, cycle: &Cycle) -> (Vec<TimedMessage>, Vec<TimedMessage>) {
        // Play note offs
        let mut sequence_out_messages = Sequencer::note_off_messages(cycle, &mut self.sequence_note_offs);
        let mut control_out_messages = Sequencer::note_off_messages(cycle, &mut self.indicator_note_offs);

        // Only output sequencer notes when playing, but output indicators on reposition aswell
        if cycle.is_rolling || cycle.was_repositioned || self.should_render {
            // Get playing sequences
            if let Some(playing_phrases) = self.playing_phrases(cycle) {
                // Output those
                // TODO - Sequence with phrases of different length
                let playing_patterns = self.playing_patterns(cycle, &playing_phrases);

                // We should always redraw on reposition or button press
                let force_redraw = cycle.was_repositioned || self.should_render;

                if let Some(note_events) = self.main_indicator_note_events(cycle, force_redraw, &playing_patterns, &playing_phrases) {
                    control_out_messages.extend(note_events);
                }

                // Get playing notes for sequencer
                let playing_notes = self.playing_notes(cycle, &playing_patterns);

                // Output note events
                let (sequence_note_offs, sequence_note_ons) 
                    = Sequencer::sequence_note_events(cycle, &playing_notes, 1, 0, None, None, None);

                if cycle.is_rolling {
                    sequence_out_messages.extend(sequence_note_ons);
                }

                // Draw dynamic indicators
                match self.overview {
                    OverView::Instrument => {
                        if let Some(note_events) = self.playable_indicator_note_events(cycle, force_redraw, &playing_patterns, &playing_phrases) {
                            control_out_messages.extend(note_events);
                        }
                    },
                    OverView::Sequence => {
                        if cycle.is_rolling {
                            let (indicator_note_offs, control_note_ons) 
                                = Sequencer::sequence_note_events(cycle, &playing_notes, 3, self.instrument_group * 8, Some(0x33), Some(1), Some(0));

                            self.indicator_note_offs.extend(indicator_note_offs);
                            control_out_messages.extend(control_note_ons);
                        }
                    }
                }

                // Always trigger draw of sequence indicator as it will always be active
                if let Some(note_events) = self.sequence_indicator_note_events(cycle, force_redraw) {
                    control_out_messages.extend(note_events);
                }

                // Also push note offs (TODO - Why after all this stuff?)
                if cycle.is_rolling {
                    self.sequence_note_offs.extend(sequence_note_offs);
                }
            }
        }

        // Return messages
        (control_out_messages, sequence_out_messages)
    }
}
