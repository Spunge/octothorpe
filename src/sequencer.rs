
use super::beats_to_ticks;
use super::cycle::Cycle;
use super::message::{Message, TimedMessage};
use super::grid::Grid;
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

    // Various indicators
    indicator_grid: Grid,
    instrument_grid: Grid,
    group_grid: Grid,
    playable_grid: Grid,
    detailview_grid: Grid,
    overview_grid: Grid,
    sequence_grid: Grid,
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

        Sequencer{
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

            // Only show in instrument overview
            playable_grid: Grid::new(1, 5, 0x52),
            instrument_grid: Grid::new(8, 1, 0x33),

            // Show in both overviews
            group_grid: Grid::new(1, 1, 0x50),
            sequence_grid: Grid::new(1, 4, 0x57),
            overview_grid: Grid::new(1, 1, 0x3A),
            detailview_grid: Grid::new(1, 1, 0x3E),
            indicator_grid: Grid::new(8, 1, 0x34),
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

    fn instrument_key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
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
            // Up / down 
            0x5E | 0x5F => {
                if let DetailView::Pattern = self.detailview {
                    let should_redraw = match message.bytes[1] {
                        0x5E => self.instrument().pattern().change_base_note(4),
                        0x5F => self.instrument().pattern().change_base_note(-4),
                        _ => false,
                    };

                    if should_redraw { 
                        let mut messages = self.instrument().pattern().playable.main_grid.clear(false);
                        messages.extend(self.instrument().pattern().draw_pattern());
                        messages
                    } else { 
                        vec![] 
                    }
                } else {
                    vec![]
                }
            },
            0x31 | 0x32 | 0x60 | 0x61 => {
                let should_redraw = match message.bytes[1] {
                    0x31 => self.playable().change_zoom((message.bytes[0] - 0x90 + 1) as u32),
                    0x32 => self.playable().change_length(message.bytes[0] - 0x90 + 1),
                    0x61 => self.playable().change_offset(-1),
                    0x60 => self.playable().change_offset(1),
                    _ => false,
                };

                if should_redraw { self.redraw_instrument() } else { vec![] }
            },
            _ => vec![],
        }
    }

    fn sequence_key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
        match message.bytes[1] {
            0x32 => {
                let group = self.group;
                self.sequence().toggle_active(message.bytes[0]- 0x90);
                self.sequence().draw_active_grid(group)
            },
            _ => vec![],
        }
    }

    pub fn shared_key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
        match self.overview {
            OverView::Instrument => self.instrument_key_pressed(message),
            OverView::Sequence => self.sequence_key_pressed(message),
        }
    }

    pub fn key_pressed(&mut self, message: jack::RawMidi) -> Vec<Message> {
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
            _ => vec![],
        }
    }

    // Key released is 0x80 + channel instead of 0x90 + channel
    pub fn key_released(&mut self, message: jack::RawMidi) -> Option<Vec<Message>> {
        self.keys_pressed.retain(|key_pressed| {
            key_pressed.channel != message.bytes[0] + 16
                || key_pressed.note != message.bytes[1]
                || key_pressed.velocity != message.bytes[2]
        });
        None
    }

    fn switch_sequence(&mut self, sequence: u8) -> Vec<Message> {
        // Clear sequence stuff
        let mut messages = self.clear_sequence(false);
        messages.extend(self.sequence_grid.clear(false));
        // If we're looking at instrument at the moment, clear that & switch to sequence
        if let OverView::Instrument = self.overview {
            messages.extend(self.clear_instrument(false));
            self.toggle_overview();
        }
        // Set new sequence & draw
        self.sequence = sequence;
        messages.extend(self.draw_sequence());
        messages
    }

    fn switch_group(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        self.group = if self.group == 1 { 0 } else { 1 };
        messages.append(&mut self.draw());
        messages
    }

    fn switch_instrument(&mut self, instrument: u8) -> Vec<Message> {
        // Clear instrument stuff
        let mut messages = self.clear_instrument(false);
        // If we're looking @ sequence, clear that and toggle
        if let OverView::Sequence = self.overview {
            messages.extend(self.clear_sequence(false));
            self.toggle_overview();
        }
        self.instrument = instrument;
        messages.append(&mut self.draw_instrument());
        messages
    }

    fn switch_playable(&mut self, playable: u8) -> Vec<Message> {
        let mut messages = self.clear_instrument(false);
        match self.detailview {
            DetailView::Pattern => { self.instrument().pattern = playable as usize },
            DetailView::Phrase => { self.instrument().phrase = playable as usize },
        }

        // Reset pattern on shift click
        if self.is_shift_pressed() {
            match self.detailview {
                DetailView::Pattern => { self.instrument().pattern().reset() },
                DetailView::Phrase => { self.instrument().phrase().reset() },
            }
        }

        messages.append(&mut self.draw_instrument());
        messages
    }

    fn toggle_overview(&mut self) {
        match self.overview {
            OverView::Instrument => { self.overview = OverView::Sequence },
            OverView::Sequence => { self.overview = OverView::Instrument },
        }
    }

    fn switch_overview(&mut self) -> Vec<Message> {
        let mut messages = self.clear(false);
        self.toggle_overview();
        messages.extend(self.draw());
        messages
    }

    fn switch_detailview(&mut self) -> Vec<Message> {
        let mut messages = self.clear_instrument(false);
        match self.detailview {
            DetailView::Pattern => { self.detailview = DetailView::Phrase },
            DetailView::Phrase => { self.detailview = DetailView::Pattern },
        }
        messages.append(&mut self.draw_instrument());
        messages
    }

    fn playable(&mut self) -> &mut Playable {
        match self.detailview {
            DetailView::Pattern => { &mut self.instrument().pattern().playable },
            DetailView::Phrase => { &mut self.instrument().phrase().playable },
        }
    }

    fn playable_led(&mut self) -> usize {
        match self.detailview {
            DetailView::Pattern => self.instrument().pattern,
            DetailView::Phrase => self.instrument().phrase,
        }
    }

    fn draw_sequencer(&mut self) -> Vec<Message> {
        vec![
            self.sequence_grid.switch_led(0, self.sequence, 1),
            self.group_grid.switch_led(0, 0, self.group),
        ]
    }

    fn clear_sequencer(&mut self, force: bool) -> Vec<Message> {
        vec![
            self.sequence_grid.clear(force),
            self.group_grid.clear(force),
            self.overview_grid.clear(force),
        ].into_iter().flatten().collect()
    }

    fn draw_instrument(&mut self) -> Vec<Message> {
        let playable_led = self.playable_led() as u8;
        // Only show detailview led when we're in instument view
        let detailview_led = match (&self.overview, &self.detailview) { 
            (OverView::Instrument, DetailView::Pattern) => 1, 
            _ => 0, 
        };

        let mut messages = match self.detailview {
            DetailView::Pattern => { self.instrument().pattern().draw() },
            DetailView::Phrase => { self.instrument().phrase().draw() },
        };

        messages.push(self.instrument_grid.switch_led(self.instrument, 0, 1));
        messages.push(self.playable_grid.switch_led(0, playable_led, 1));
        messages.push(self.detailview_grid.switch_led(0, 0, detailview_led));
        let overview_state = match self.overview { OverView::Instrument => 1, _ => 0, };
        messages.push(self.overview_grid.switch_led(0, 0, overview_state));
        messages
    }

    fn redraw_instrument(&mut self) -> Vec<Message> {
        match self.detailview {
            DetailView::Pattern => { 
                let mut messages = self.instrument().pattern().clear(false);
                messages.extend(self.instrument().pattern().draw());
                messages
            },
            DetailView::Phrase => { 
                let mut messages = self.instrument().phrase().clear(false);
                messages.extend(self.instrument().phrase().draw());
                messages
            },
        }
    }

    fn clear_instrument(&mut self, force: bool) -> Vec<Message> {
        vec![
            self.instrument_grid.clear(force),
            self.playable_grid.clear(force),
            self.detailview_grid.clear(force),
            match self.detailview {
                DetailView::Pattern => { self.instrument().pattern().clear(force) },
                DetailView::Phrase => { self.instrument().phrase().clear(force) },
            },
        ].into_iter().flatten().collect()
    }

    fn draw_sequence(&mut self) -> Vec<Message> {
        let group = self.group;
        let mut messages = self.sequence().draw_sequence(group);
        messages.extend(self.sequence().draw_active_grid(group));
        messages.push(self.overview_grid.switch_led(0, 0, match self.overview { OverView::Instrument => 1, _ => 0, }));
        messages.push(self.sequence_grid.switch_led(0, self.sequence, 1));
        messages
    }

    fn clear_sequence(&mut self, force: bool) -> Vec<Message> {
        let mut messages = self.sequence().main_grid.clear(force);
        messages.extend(self.sequence().active_grid.clear(force));
        messages
    }
   
    fn draw_overview(&mut self) -> Vec<Message> {
        match self.overview {
            OverView::Instrument => self.draw_instrument(),
            OverView::Sequence => self.draw_sequence(),
        }
    }
    
    fn clear_overview(&mut self, force: bool) -> Vec<Message> {
        match self.overview {
            OverView::Instrument => self.clear_instrument(force),
            OverView::Sequence => self.clear_sequence(force),
        }
    }

    // Draw all the things
    pub fn draw(&mut self) -> Vec<Message> {
        let mut messages = self.draw_sequencer();
        messages.extend(self.draw_overview());
        messages
    }

    pub fn clear(&mut self, force: bool) -> Vec<Message> {
        let mut messages = self.clear_sequencer(force);
        messages.extend(self.clear_overview(force));
        messages
    }

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

    pub fn output(&mut self, cycle: &Cycle) -> (Vec<TimedMessage>, Vec<TimedMessage>) {
        // First notes off
        let mut messages = self.note_off_messages(cycle);
        let mut control_messages = self.control_off_messages(cycle);
        
        // Next notes on
        if cycle.is_rolling {
            if let Some(sequences) = self.playing_sequences(cycle) {
                // Get phrases that are playing in sequence
                // ( instrument, phrase )
                let phrases: Vec<(usize, usize)> = sequences.iter()
                //sequences.iter()
                    .flat_map(|sequence| self.sequences[*sequence].playing_phrases())
                    .collect();
                    
                // Get patterns that are playingg
                // Instrument & played pattern
                let patterns: Vec<(usize, PlayedPattern)> = phrases.iter()
                    .flat_map(|(instrument, phrase)| {
                        self.instruments[*instrument].phrases[*phrase]
                            .playing_patterns(cycle, &self.instruments[*instrument].patterns)
                            .into_iter()
                            .map(move |played_pattern| {
                                (*instrument, played_pattern)
                            })
                    })
                    .collect();

                let notes: Vec<(TimedMessage, NoteOff)> = patterns.iter()
                    .flat_map(|(instrument, played_pattern)| {
                        self.instruments[*instrument].patterns[played_pattern.index]
                            .playing_notes(cycle, played_pattern.start, played_pattern.end)
                    })
                    .collect();

                notes.into_iter()
                    .for_each(|(message, note_off)| {
                        // Remember control off so we can show sequence indicator
                        self.note_offs.push(note_off);
                        messages.push(message);
                    });

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
            }
        }

        // Draw pattern or phrase indicator
        if let OverView::Instrument = self.overview {
            control_messages.extend(self.draw_indicator(cycle));
        }

        (control_messages, messages)
    }
}
