
use std::ops::Range;
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
use super::instrument::Instrument;
use super::phrase::{Phrase, PlayedPattern};
use super::pattern::Pattern;
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
    group: u8,

    sequence_note_offs: Vec<(u32, Message)>,
    indicator_note_offs: Vec<(u32, Message)>,
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
            sequence_note_offs: vec![],
            indicator_note_offs: vec![],

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
            // TODO - when shortening length, notes or phrases that are longer as playable length
            // should be cut shorter aswell
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
            0x50 => self.switch_group(),
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

    fn switch_group(&mut self) {
        self.group = if self.group == 1 { 0 } else { 1 };
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

            if let OverView::Sequence = self.overview {
                self.switch_overview();
            }
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
            OverView::Instrument => self.overview = OverView::Sequence,
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
        self.state_current = [9; 92];
        self.state_next = [0; 92];

        self.should_render = true;
    }

    // Output a message for each changed state in the grid
    pub fn output_grid(&self, range: Range<usize>, y: u8) -> Vec<Message> {
        let mut output = vec![];
        let start = range.start;

        for index in range {
            if self.state_current[index] != self.state_next[index] {
                let led = (index - start) as u8;
                let x = if self.state_next[index] == 0 { 0x80 } else { 0x90 };

                output.push(Message::Note([x + led % 8, y + led / 8, self.state_next[index]]));
            }
        }

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

    pub fn output_static_leds(&mut self) -> Vec<Message> {
        let mut output = vec![];
    
        // Draw if we have to
        if self.should_render {
            self.draw_main_grid();
            self.draw_green_grid();
            self.draw_blue_grid();
            self.draw_instruments_grid();
            self.draw_group_button();
            self.draw_detailview_button();

            output.extend(self.output_grid(self.index_green.clone(), 0x32));
            output.extend(self.output_grid(self.index_blue.clone(), 0x31));
            output.extend(self.output_grid(self.index_main.clone(), 0x35));
            output.extend(self.output_grid(self.index_instruments.clone(), 0x33));
            output.extend(self.output_button(self.index_group, 0x50));
            output.extend(self.output_button(self.index_detailview, 0x3E));

            // Switch buffer
            self.state_current = self.state_next;
            self.should_render = false;
        }
 
        output
    }

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

    fn active_pattern_indicator_leds(&self, cycle: &Cycle, played_patterns: &Vec<(usize, PlayedPattern)>) -> Vec<(u32, u32)> {
       played_patterns.into_iter()
            // Get playing patterns for currently shown instrument && pattern
            .filter(|(instrument, played_pattern)| {
                *instrument == self.instrument as usize 
                    && played_pattern.index == self.instruments[*instrument].pattern
            })
            // Get 
            .flat_map(|(instrument, played_pattern)| {
                let pattern_length = played_pattern.end - played_pattern.start;
                // Get ticks per led
                let ticks_per_led = self.instruments[*instrument].patterns[played_pattern.index].playable.ticks_per_led();
                let ticks_offset = self.instruments[*instrument].patterns[played_pattern.index].playable.ticks_offset();

                // Loop over the played patterns range to see if one of the leds should activate
                (played_pattern.start..played_pattern.end).step_by(ticks_per_led as usize)
                    .filter_map(move |tick| {
                        cycle.delta_frames(tick + ticks_offset).and_then(|frame| {
                            // Return frame to switch at and led to switch
                            Some((frame, (tick - played_pattern.start) / ticks_per_led))
                        })
                    })
            })
            .collect()
    }

    fn set_playing_sequence(&mut self, sequence: usize) {
        self.sequence_playing = sequence;
        self.sequence_queued = None;
    }

    fn current_playing_sequences(&self, cycle: &Cycle) -> Option<Vec<usize>> {
        // Get current sequence as we definitely have to play that
        let current_sequence = &self.sequences[self.sequence_playing];
        // Can we play current sequence, could be it has length 0 for no phrases playing
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

    fn playing_patterns(&self, cycle: &Cycle, sequences: Vec<usize>) -> Vec<(usize, PlayedPattern)> {
        sequences.iter()
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
            .collect()
    }

    // Get notes that should be triggered in currently playing sequences
    fn playing_notes(&self, cycle: &Cycle, played_patterns: &Vec<(usize, PlayedPattern)>) -> Vec<(u32, &Note)> {
        // Get phrases that are playing in sequence
        // ( instrument, phrase )
        played_patterns.into_iter()
            // Next, get notes for each instrument / played pattern
            .flat_map(|(instrument, played_pattern)| {
                self.instruments[*instrument].patterns[played_pattern.index]
                    .playing_notes(cycle, played_pattern.start, played_pattern.end)
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

    fn sequence_note_events(cycle: &Cycle, notes: &Vec<(u32, &Note)>) -> (Vec<(u32, Message)>, Vec<TimedMessage>) {
        let note_offs: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let length = (note.end - note.start);
                let tick = cycle.absolute_start + delta_ticks;

                (tick + length, note.message(0x80, None, None))
            })
            .collect();

        let note_ons: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let delta_frames = (*delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;
                TimedMessage::new(delta_frames, note.message(0x90, None, None))
            })
            .collect();

        (note_offs, note_ons)
    }

    fn sequence_indicator_note_events(cycle: &Cycle, notes: &Vec<(u32, &Note)>) -> (Vec<(u32, Message)>, Vec<TimedMessage>) {
        let note_offs: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let length = (note.end - note.start);
                let tick = cycle.absolute_start + delta_ticks;

                (tick + length / 3, note.message(0x90, Some(0x33), Some(0)))
            })
            .collect();

        let note_ons: Vec<_> = notes.iter()
            .map(|(delta_ticks, note)| {
                let delta_frames = (*delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;
                TimedMessage::new(delta_frames, note.message(0x90, Some(0x33), Some(1)))
            })
            .collect();

        (note_offs, note_ons)
    }


    // Send midi to process handler
    pub fn output_midi(&mut self, cycle: &Cycle) -> (Vec<TimedMessage>, Vec<TimedMessage>) {
        // Play note offs
        let mut sequence_out_messages = Sequencer::note_off_messages(cycle, &mut self.sequence_note_offs);
        let mut control_out_messages = Sequencer::note_off_messages(cycle, &mut self.indicator_note_offs);

        // Next notes on
        if cycle.is_rolling {
            // Get playing sequences
            if let Some(sequences) = self.playing_sequences(cycle) {
                // Output those
                let played_patterns = self.playing_patterns(cycle, sequences);
                let notes = self.playing_notes(cycle, &played_patterns);

                let (sequence_note_offs, sequence_note_ons) = Sequencer::sequence_note_events(cycle, &notes);
                sequence_out_messages.extend(sequence_note_ons);

                match self.overview {
                    OverView::Instrument => {
                        match self.detailview {
                            DetailView::Pattern => {
                                println!("{:?}", self.active_pattern_indicator_leds(cycle, &played_patterns));
                            },
                            DetailView::Phrase => {},
                        }
                    },
                    OverView::Sequence => {
                        let (indicator_note_offs, control_note_ons) = Sequencer::sequence_indicator_note_events(cycle, &notes);
                        self.indicator_note_offs.extend(indicator_note_offs);
                        control_out_messages.extend(control_note_ons);
                    }
                }

                self.sequence_note_offs.extend(sequence_note_offs);
            }
        }

        // Return messages
        (control_out_messages, sequence_out_messages)
    }
}
