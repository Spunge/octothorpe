
use super::TickRange;
use super::cycle::*;
use super::message::{Message, TimedMessage};
use super::instrument::Instrument;
use super::sequence::Sequence;
use super::loopable::*;

#[derive(Debug, PartialEq, Copy, Clone)]
struct PlayingSequence {
    // Start tick
    start: u32,
    index: usize,
}

impl PlayingSequence {
    fn new(start: u32, index: usize) -> Self { 
        Self { start, index }
    }
}

pub struct TimeLine {
    playing_sequences: Vec<PlayingSequence>,
}

impl TimeLine {
    fn new()  -> Self {
        Self { 
            playing_sequences: vec![PlayingSequence::new(0, 0)] 
        }
    }

    fn playing_sequence(&self, start: u32) -> &PlayingSequence {
        self.playing_sequences.iter()
            .filter(|playing_sequence| playing_sequence.start <= start)
            .max_by_key(|playing_sequence| playing_sequence.start)
            .unwrap()
    }

    fn next_sequence(&mut self, playing_sequence: &PlayingSequence, sequence_length: u32) -> PlayingSequence {
        let next_sequence_tick = playing_sequence.start + sequence_length;
        let sequence = self.playing_sequence(next_sequence_tick);

        // Is next sequence still this one?
        if sequence.start == playing_sequence.start {
            let next_sequence = PlayingSequence::new(next_sequence_tick, playing_sequence.index);
            self.playing_sequences.push(next_sequence);
            next_sequence
        } else {
            *sequence
        }
    }
}

pub struct Sequencer {
    pub instruments: [Instrument; 16],
    pub sequences: [Sequence; 4],
    
    pub timeline: TimeLine,
}

impl Sequencer {
    pub fn new(client: &jack::Client) -> Self {
        // Build instruments array, shame there's no way to do this elegantly without a macro as far as i can tell
        let instruments = [
            Instrument::new(client, 1),
            Instrument::new(client, 2),
            Instrument::new(client, 3),
            Instrument::new(client, 4),
            Instrument::new(client, 5),
            Instrument::new(client, 6),
            Instrument::new(client, 7),
            Instrument::new(client, 8),
            Instrument::new(client, 9),
            Instrument::new(client, 10),
            Instrument::new(client, 11),
            Instrument::new(client, 12),
            Instrument::new(client, 13),
            Instrument::new(client, 14),
            Instrument::new(client, 15),
            Instrument::new(client, 16),
        ];

        // Build sequence we can trigger
        let sequences = [
            Sequence::new(),
            Sequence::new(),
            Sequence::new(),
            Sequence::new(), 
        ];

        Sequencer {
            instruments,
            sequences,
            timeline: TimeLine::new(),
        }
    }

    pub fn get_instrument(&mut self, index: usize) -> &mut Instrument {
        &mut self.instruments[index]
    }

    pub fn get_sequence(&mut self, index: usize) -> &mut Sequence {
        &mut self.sequences[index]
    }

    // TODO - Direct queueing
    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        let playing_sequence = *self.timeline.playing_sequence(cycle.tick_range.start);
        let playing_sequence_length = self.sequences[playing_sequence.index].length(&self.instruments);
        let sequence_stop = playing_sequence.start + playing_sequence_length;

        if ! cycle.is_rolling {
            return
        }

        //for (instrument_index, instrument) in self.instruments.iter_mut().enumerate() {
        if let Some((instrument_index, instrument)) = self.instruments.iter_mut().enumerate().next() {
            let mut starting_notes = vec![];

            let sequence_playing = &self.sequences[playing_sequence.index];
            let playing_phrase = sequence_playing.get_phrase(instrument_index);

            // Insert currenly playing cycle into timeline when there's no next cycle queued
            if cycle.tick_range.contains(sequence_stop) || sequence_stop < cycle.tick_range.start {
                self.timeline.next_sequence(&playing_sequence, playing_sequence_length);
            }

            if cycle.tick_range.contains(sequence_stop) {
                if let (Some(index), true) = (playing_phrase, cycle.tick_range.start < sequence_stop) {
                    // Add from start to sequence_stop
                    starting_notes.extend(instrument.starting_notes(TickRange::new(cycle.tick_range.start, sequence_stop), playing_sequence.start, index));
                }

                let next_sequence = self.timeline.next_sequence(&playing_sequence, playing_sequence_length);
                if let Some(index) = self.sequences[next_sequence.index].get_phrase(instrument_index) {
                    // Only queue more of this when nothing is queued
                    starting_notes.extend(instrument.starting_notes(TickRange::new(sequence_stop, cycle.tick_range.stop), sequence_stop, index));
                }
            } else {
                if let Some(index) = playing_phrase {
                    starting_notes.extend(instrument.starting_notes(cycle.tick_range, playing_sequence.start, index))
                }
            }

            instrument.output_midi(cycle, starting_notes);
        }
    }

    // One of the control knobs on the APC was turned
    /*
    pub fn knob_turned(&mut self, time: u32, knob: u8, value: u8) {
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
        // TODO - Output this to corresponding port
        //vec![TimedMessage::new(time, Message::Note([0xB0 + out_channel, out_knob, value]))]
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
        None
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

    pub fn queue_sequence(&mut self, sequence: u8) {
        // TODO 
        //self.sequence_queued = Some(sequence as usize);
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
        if cycle.is_rolling || cycle.was_repositioned {
            // Get playing sequences
            if let Some(playing_phrases) = self.playing_phrases(cycle) {
                // Output those
                // TODO - Sequence with phrases of different length
                let playing_patterns = self.playing_patterns(cycle, &playing_phrases);

                // We should always redraw on reposition or button press
                let force_redraw = cycle.was_repositioned;

                //if let Some(note_events) = self.main_indicator_note_events(cycle, force_redraw, &playing_patterns, &playing_phrases) {
                //control_out_messages.extend(note_events);
                //}

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
                        //if let Some(note_events) = self.playable_indicator_note_events(cycle, force_redraw, &playing_patterns, &playing_phrases) {
                        //control_out_messages.extend(note_events);
                        //}
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
                //if let Some(note_events) = self.sequence_indicator_note_events(cycle, force_redraw) {
                //control_out_messages.extend(note_events);
                //}

                // Also push note offs (TODO - Why after all this stuff?)
                if cycle.is_rolling {
                    self.sequence_note_offs.extend(sequence_note_offs);
                }
            }
        }

        // Return messages
        (control_out_messages, sequence_out_messages)
    }
    */
}
