
use std::ops::Range;
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
use super::instrument::Instrument;
use super::phrase::{Phrase, PlayedPattern};
use super::pattern::Pattern;
use super::sequence::Sequence;
use super::playable::Playable;
use super::note::NoteOff;

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
    group: u8,

    note_offs: Vec<NoteOff>,
    control_offs: Vec<(u32, u8)>,
    keys_pressed: Vec<KeyPress>,

    instruments: [Instrument; 16],
    instrument: u8,

    sequences: [Sequence; 4],
    sequence: u8,

    // What is playing?
    sequence_playing: usize,
    sequence_queued: Option<usize>,

    // What are we showing?
    overview: OverView,
    detailview: DetailView,

    pub should_render: bool,
    state_current: [u8; 92],
    state_next: [u8; 92],
    
    // Dynamic
    index_indicator: Range<usize>,
    index_playables: Range<usize>,
    index_sequences: Range<usize>,
    // Buttons
    index_group: usize,
    index_detailview: usize,
    index_overview: usize,
    // Static
    index_instruments: Range<usize>,
    index_main: Range<usize>,
    index_green: Range<usize>,
    index_blue: Range<usize>,
    index_red: Range<usize>,
}

impl Sequencer {
    pub fn new() -> Self {
        // Build instruments for each midi channel
        let mut instruments = [
            Instrument::new(0), Instrument::new(1), Instrument::new(2), Instrument::new(3),
            Instrument::new(4), Instrument::new(5), Instrument::new(6), Instrument::new(7),
            Instrument::new(8), Instrument::new(9), Instrument::new(10), Instrument::new(11),
            Instrument::new(12), Instrument::new(13), Instrument::new(14), Instrument::new(15),
        ];
        instruments[0].patterns[0] = Pattern::default(0);
        instruments[0].phrases[0] = Phrase::default();
    
        // Build sequence we can trigger
        let sequences = [ Sequence::default(), Sequence::alternate_default(), Sequence::new(), Sequence::new(), ];

        Sequencer {
            instruments,
            instrument: 0,
            group: 0,

            keys_pressed: vec![],
            note_offs: vec![],
            control_offs: vec![],

            sequences,
            sequence: 0,

            sequence_playing: 0,
            sequence_queued: Some(0),

            // What are we currently showing?
            detailview: DetailView::Phrase,
            overview: OverView::Instrument,

            should_render: false,
            state_current: [0; 92],
            state_next: [0; 92],

            // Range of buttons in states buffer
            // Dynamic
            index_indicator: 40..48,
            index_playables: 80..85,
            index_sequences: 85..89,
            // Static
            index_instruments: 48..56,
            index_main: 0..40,
            index_green: 56..64,
            index_blue: 64..72,
            index_red: 72..80,
            // Static buttons
            index_group: 89,
            index_detailview: 90,
            index_overview: 91,
        }
    }

    fn instrument(&mut self) -> &mut Instrument {
        &mut self.instruments[(self.group * 8 + self.instrument) as usize]
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
            0x52 ... 0x56 => self.switch_playable(message.bytes[1] - 0x52),
            // TODO - Grid should add notes & add phrases
            0x35 ... 0x39 => {
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
                    DetailView::Pattern => self.instrument().pattern().toggle_note(from..to, message.bytes[1] - 0x35),
                    DetailView::Phrase => self.instrument().phrase().toggle_pattern(from..to, message.bytes[1] - 0x35),
                }
            },
            0x5E => self.instrument().pattern().change_base_note(4),
            0x5F => self.instrument().pattern().change_base_note(-4),
            0x31 => self.playable().change_zoom((message.bytes[0] - 0x90 + 1) as u32),
            0x32 => self.playable().change_length(message.bytes[0] - 0x90 + 1),
            0x61 => self.playable().change_offset(-1),
            0x60 => self.playable().change_offset(1),
            _ => (),
        }
    }

    fn sequence_key_pressed(&mut self, message: jack::RawMidi) {
        match message.bytes[1] {
            0x32 => self.sequence().toggle_active(message.bytes[0] - 0x90),
            0x35 ... 0x39 => {
                let instrument = message.bytes[0] - 0x90 + self.group * 8;
                self.sequence().toggle_phrase(instrument, message.bytes[1] - 0x35);
            },
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
        // Remember remember
        self.keys_pressed.push(KeyPress::new(message));

        match message.bytes[1] {
            //0x30 => self.sequencer.toggle_instrument_active(message.bytes[0] - 0x90),
            0x50 => self.switch_group(),
            0x3A => self.switch_overview(),
            0x33 => self.switch_instrument(message.bytes[0] - 0x90),
            0x57 | 0x58 | 0x59 | 0x5A => self.switch_sequence(message.bytes[1] - 0x57),
            0x3E | 0x31 | 0x32 | 0x60 | 0x61 | 0x51 | 0x5E | 0x5F => self.shared_key_pressed(message),
            // Playable select
            0x52 ... 0x56 => self.shared_key_pressed(message),
            // Main grid
            0x35 ... 0x39 => self.shared_key_pressed(message),
            _ => (),
        };

        self.should_render = true;
    }

    // Key released is 0x80 + channel instead of 0x90 + channel
    pub fn key_released(&mut self, message: jack::RawMidi) {
        self.keys_pressed.retain(|key_pressed| {
            key_pressed.channel != message.bytes[0] + 16
                || key_pressed.note != message.bytes[1]
                || key_pressed.velocity != message.bytes[2]
        });
    }

    fn switch_sequence(&mut self, sequence: u8) {
        // Queue sequence
        if self.is_shift_pressed() {
            self.sequence_queued = Some(sequence as usize);
        }
        // If we're looking at instrument at the moment, clear that & switch to sequence
        if let OverView::Instrument = self.overview {
            self.switch_overview();
        }
        // Set new sequence & draw
        self.sequence = sequence;
    }

    fn switch_group(&mut self) {
        self.group = if self.group == 1 { 0 } else { 1 };
    }

    fn switch_instrument(&mut self, instrument: u8) {
        // If we're looking @ sequence, clear that and toggle
        if let OverView::Sequence = self.overview {
            self.switch_overview();
        }
        self.instrument = instrument;
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
                DetailView::Phrase => self.instrument().phrase().reset(),
            }
        }
    }

    fn switch_overview(&mut self) {
        match self.overview {
            OverView::Instrument => self.overview = OverView::Sequence,
            OverView::Sequence => self.overview = OverView::Instrument,
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
        self.state_current = [9; 92];
        self.state_next = [0; 92];

        self.should_render = true;
    }

    // Output a message for each changed state in the grid
    pub fn output_grid(&self, range: Range<usize>, x: u8, y: u8) -> Vec<Message> {
        let mut output = vec![];
        let start = range.start;

        for index in range {
            if self.state_current[index] != self.state_next[index] {
                let led = (index - start) as u8;

                output.push(Message::Note([x + led % 8, y + led / 8, self.state_next[index]]));
            }
        }

        output
    }

    pub fn output_button(&self, index: usize, x: u8, y: u8) -> Option<Message> {
        if self.state_current[index] != self.state_next[index] {
            Some(Message::Note([x, y, self.state_next[index]]))
        } else {
            None
        }
    }

    fn draw_main_grid(&mut self) {
        // Why do i have to do this?
        let group = self.group;

        // Clear grid
        for index in self.index_main.clone() {
            self.state_next[index] = 0;
        }

        let states = match self.overview {
            OverView::Instrument => match self.detailview {
                DetailView::Pattern => self.instrument().pattern().led_states(),
                DetailView::Phrase => self.instrument().phrase().led_states(),
            }
            OverView::Sequence => self.sequence().led_states(group),
        };

        // Get states that are within grid
        let valid_states = states.into_iter().filter(|(x, y, _)| {
            x < &8 && x >= &0 && y < &5 && y >= &0
        });

        for (x, y, state) in valid_states {
            self.state_next[y as usize * 8 + x as usize] = state;
        }
    }

    fn draw_green_grid(&mut self) {
        for index in self.index_green.clone() {
            let led = index - self.index_green.start;

            self.state_next[index] = match self.overview {
                // In instrument, green grid shows length of playable
                OverView::Instrument => {
                    let length = (self.playable().ticks / self.playable().minimum_ticks) as usize;
                    if led < length { 1 } else { 0 }
                },
                // In Sequence, green grid shows active instruments
                OverView::Sequence => {
                    let instrument = self.group as usize * 8 + led;
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
            let instrument = self.group * 8 + self.instrument;

            self.state_next[index] = match self.overview {
                OverView::Instrument => if led as u8 == instrument { 1 } else { 0 },
                _ => 0,
            };
        }
    }

    fn draw_group_button(&mut self) {
        self.state_next[self.index_group] = self.group;
    }

    fn draw_overview_button(&mut self) {
        self.state_next[self.index_overview] = match self.overview {
            OverView::Instrument => 1,
            _ => 0,
        };
    }

    fn draw_detailview_button(&mut self) {
        self.state_next[self.index_detailview] = match self.overview {
            OverView::Instrument => match self.detailview { DetailView::Pattern => 1, _ => 0 },
            _ => 0,
        };
    }

    //fn draw_sequences_grid(&mut self) {
    //}
    //fn draw_playables_grid(&mut self) {
    //}
    //fn draw_indicator_grid(&mut self) {
    //}

    pub fn output_static_control(&mut self) -> Vec<Message> {
        let mut output = vec![];
    
        // Draw if we have to
        if self.should_render {
            self.draw_main_grid();
            self.draw_green_grid();
            self.draw_blue_grid();
            self.draw_instruments_grid();
            self.draw_group_button();
            self.draw_overview_button();
            self.draw_detailview_button();

            output.extend(self.output_grid(self.index_green.clone(), 0x90, 0x32));
            output.extend(self.output_grid(self.index_blue.clone(), 0x90, 0x31));
            output.extend(self.output_grid(self.index_main.clone(), 0x90, 0x35));
            output.extend(self.output_grid(self.index_instruments.clone(), 0x90, 0x33));
            output.extend(self.output_button(self.index_group, 0x90, 0x50));
            output.extend(self.output_button(self.index_overview, 0x90, 0x3A));
            output.extend(self.output_button(self.index_detailview, 0x90, 0x3E));

            // Switch buffer
            self.state_current = self.state_next;
            self.should_render = false;
        }
 
        output
    }

    pub fn output_control(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let messages = self.output_static_control();

        messages.into_iter().map(|message| TimedMessage::new(0, message)).collect()
    }


    /*
    pub fn draw_active_grid(&mut self, group: u8) -> Vec<Message> {
        let leds = self.active_grid.width;

        (0..leds)
            .map(|led| {
                let is_active = self.active[(led + group * 8) as usize];
                let state = if is_active { 1 } else { 0 };
                self.active_grid.switch_led(led, 0, state)
            })
            .collect()
    }
    */

    /*
    fn draw_indicator_grid(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        if let Some(sequences) = self.playing_sequences(cycle) {
            // Get phrases that are playing in sequence
            // ( instrument, phrase )
            let phrases: Vec<(usize, usize)> = sequences.iter()
                .flat_map(|sequence| self.sequences[*sequence].playing_phrases())
                .collect();

            match self.detailview {
                DetailView::Pattern => {
                    // Get patterns that are playingg
                    // Instrument & played pattern
                    let mut patterns: Vec<(usize, PlayedPattern)> = phrases.iter()
                        .flat_map(|(instrument, phrase)| {
                            self.instruments[*instrument].phrases[*phrase]
                                .playing_patterns(cycle, &self.instruments[*instrument].patterns)
                                .into_iter()
                                .map(move |played_pattern| {
                                    (*instrument, played_pattern)
                                })
                        })
                        .collect();

                    patterns = patterns.into_iter()
                        .filter(|(instrument, played_pattern)| {
                            *instrument == self.instrument as usize && played_pattern.index == self.instrument().pattern
                        })
                        .collect();

                    let ticks_offset = self.playable().ticks_offset();
                    let ticks_per_led = self.playable().ticks_per_led();

                    let mut messages = vec![];

                    for (_, played_pattern) in patterns.iter() {
                        let pattern_length = played_pattern.end - played_pattern.start;

                        for increment in 0..(pattern_length / ticks_per_led) {
                            let tick = played_pattern.start + increment * ticks_per_led + ticks_offset;
                        
                            if let Some(frames) = cycle.delta_frames(tick) {
                                messages.extend(self.indicator_grid.clear(false).into_iter().map(|message| TimedMessage::new(frames, message)));

                                if let Some(message) = self.indicator_grid.try_switch_led(increment as i32, 0, 1) {
                                    messages.push(TimedMessage::new(frames, message));
                                }
                            }
                        }
                    }

                    messages
                },
                DetailView::Phrase => {
                    let should_draw = phrases.iter()
                        .any(|(instrument, phrase)| {
                            *instrument == self.instrument as usize && *phrase == self.instrument().phrase
                        });

                    if should_draw {
                        //println!("should draw phrase");
                        vec![]
                    } else {
                        vec![]
                    }
                }
            }
        } else {
            vec![]
        }
    }
    */

    /*
    fn draw_indicator(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut messages = vec![];

        if cycle.was_repositioned || cycle.is_rolling {
            if cycle.was_repositioned {
                let beat_start = (cycle.start / beats_to_ticks(1.0)) * beats_to_ticks(1.0) as u32;
                let reposition_cycle = cycle.repositioned(beat_start);

                messages.extend(self.draw_indicator_grid(&reposition_cycle));
            }

            // Update grid when running, after repositioning
            if cycle.is_rolling {
                messages.extend(self.draw_indicator_grid(cycle));
            }
        }

        messages
    }
    */

    fn set_playing_sequence(&mut self, sequence: usize) {
        self.sequence_playing = sequence;
        self.sequence_queued = None;
    }

    fn current_playing_sequences(&self, cycle: &Cycle) -> Option<Vec<usize>> {
        // Get current sequence as we definitely have to play that
        let current_sequence = &self.sequences[self.sequence_playing];
        // Can we play current sequence?
        if let Some(ticks) = current_sequence.ticks(&self.instruments) {
            // If we can, play it
            let mut sequences = vec![];
            sequences.push(self.sequence_playing);

            // If cycle covers current sequence and next
            if cycle.start % ticks > cycle.end % ticks {
                if let Some(index) = self.sequence_queued {
                    //self.set_playing_sequence(index);
                    sequences.push(index);
                } else {
                    sequences.push(self.sequence_playing);
                }
            }

            Some(sequences)
        } else {
            None
        }
    }

    fn playing_sequences(&mut self, cycle: &Cycle) -> Option<Vec<usize>> {
         if let Some(sequences) = self.current_playing_sequences(cycle) {
            if sequences.len() > 1 {
                if sequences[0] != sequences[1] {
                    self.set_playing_sequence(sequences[1]);
                }
            }

            Some(sequences)
        } else {
            if let Some(index) = self.sequence_queued {
                self.set_playing_sequence(index);
                self.current_playing_sequences(cycle)
            } else {
                None
            }
        }
    }

    pub fn note_off_messages(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut messages = vec![];

        self.note_offs.retain(|note_off| {
            match cycle.delta_frames_absolute(note_off.tick) {
                Some(frames) => {
                    messages.push(TimedMessage::new(frames, note_off.message()));
                    false
                },
                None => true
            }
        });

        messages
    }

    pub fn control_off_messages(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut messages = vec![];

        self.control_offs.retain(|(tick, channel)| {
            match cycle.delta_frames_absolute(*tick) {
                Some(frames) => {
                    messages.push(TimedMessage::new(frames, Message::Note([0x90 + *channel, 0x34, 0])));
                    false
                },
                None => true
            }
        });

        messages
    }

    /*
    fn draw_sequence_indicator(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        let mut control_messages = self.control_off_messages(cycle);
        // Draw sequence indicator
        if let OverView::Sequence = self.overview {
            let sequence_indications: Vec<(TimedMessage, (u32, u8))> = patterns.iter()
                .flat_map(|(instrument, played_pattern)| {
                    self.instruments[*instrument].patterns[played_pattern.index]
                        .playing_indicators(cycle, played_pattern.start, played_pattern.end)
                })
                .collect();

            sequence_indications.into_iter()
                .for_each(|(message, note_off)| {
                    // Remember control off so we can show sequence indicator
                    self.control_offs.push(note_off);
                    control_messages.push(message);
                });
        }

        control_messages
    }
    */

    pub fn output_midi(&mut self, cycle: &Cycle) -> Vec<TimedMessage> {
        // First notes off
        let mut messages = self.note_off_messages(cycle);
        
        // Next notes on
        if cycle.is_rolling {
            if let Some(sequences) = self.playing_sequences(cycle) {
                // Get phrases that are playing in sequence
                // ( instrument, phrase )
                let notes: Vec<(TimedMessage, NoteOff)> = sequences.iter()
                    .flat_map(|sequence| self.sequences[*sequence].playing_phrases())
                    // Get patterns that are playing for Instrument & played pattern
                    .flat_map(|(instrument, phrase)| {
                        self.instruments[instrument].phrases[phrase]
                            .playing_patterns(cycle, &self.instruments[instrument].patterns)
                            .into_iter()
                            .map(move |played_pattern| {
                                (instrument, played_pattern)
                            })
                    })
                    // Next, get notes for each instrument / played pattern
                    .flat_map(|(instrument, played_pattern)| {
                        self.instruments[instrument].patterns[played_pattern.index]
                            .playing_notes(cycle, played_pattern.start, played_pattern.end)
                    })
                    .collect();

                notes.into_iter()
                    .for_each(|(message, note_off)| {
                        // Remember control off so we can show sequence indicator
                        self.note_offs.push(note_off);
                        messages.push(message);
                    });
            }
        }

        messages
    }
}
