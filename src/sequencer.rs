
use super::TickRange;
use super::cycle::*;
use super::track::Track;
use super::sequence::Sequence;
use super::loopable::*;
use super::events::*;

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

    fn playing_sequence(&self, start: u32) -> Option<&PlayingSequence> {
        self.playing_sequences.iter()
            .filter(|playing_sequence| playing_sequence.start <= start)
            .max_by_key(|playing_sequence| playing_sequence.start)
    }

    fn queue_sequence(&mut self, start: u32, sequence_index: usize) {
        self.playing_sequences.push(PlayingSequence::new(start, sequence_index));
    }
}

pub struct Sequencer {
    pub tracks: [Track; 16],
    pub sequences: [Sequence; 5],

    pub timeline: TimeLine,
}

impl Sequencer {
    pub fn new(client: &jack::Client) -> Self {
        // Build tracks array, shame there's no way to do this elegantly without a macro as far as i can tell
        let tracks = [
            Track::new(client, 1),
            Track::new(client, 2),
            Track::new(client, 3),
            Track::new(client, 4),
            Track::new(client, 5),
            Track::new(client, 6),
            Track::new(client, 7),
            Track::new(client, 8),
            Track::new(client, 9),
            Track::new(client, 10),
            Track::new(client, 11),
            Track::new(client, 12),
            Track::new(client, 13),
            Track::new(client, 14),
            Track::new(client, 15),
            Track::new(client, 16),
        ];

        // Build sequence we can trigger
        let sequences = [
            Sequence::new(),
            Sequence::new(),
            Sequence::new(),
            Sequence::new(),
            Sequence::new(),
        ];

        Sequencer {
            tracks,
            sequences,
            timeline: TimeLine::new(),
        }
    }

    pub fn track_mut(&mut self, index: usize) -> &mut Track {
        &mut self.tracks[index]
    }

    pub fn track(&self, index: usize) -> &Track {
        &self.tracks[index]
    }

    pub fn get_sequence(&mut self, index: usize) -> &mut Sequence {
        &mut self.sequences[index]
    }

    pub fn start(&mut self, cycle: &ProcessCycle) {
        // Start playing notes, as it could be we halted mid track
        self.tracks.iter_mut().for_each(|track| {
            track.start_playing_notes(cycle);
        });

        cycle.client.transport_start();
    }

    pub fn stop(&mut self, cycle: &ProcessCycle) {
        cycle.client.transport_stop();

        // Output start of playing notes, as it could be we're starting mid track
        self.tracks.iter_mut().for_each(|track| {
            track.stop_playing_notes(cycle);
        });
    }

    pub fn reset(&mut self, cycle: &ProcessCycle) {
        // Reset position back to 0
        cycle.client.transport_reposition(jack::Position::default());

        // Clear playing notes
        self.tracks.iter_mut().for_each(|track| {
            track.clear_playing_notes();
        });
    }

    pub fn autoqueue_next_sequence(&mut self, cycle: &ProcessCycle) {
        let playing_sequence = *self.timeline.playing_sequence(cycle.tick_range.start).unwrap();
        let playing_sequence_length = self.sequences[playing_sequence.index].length(&self.tracks);

        let next_start = playing_sequence.start + playing_sequence_length;

        if cycle.tick_range.contains(next_start) {
            let next_sequence = self.timeline.playing_sequence(next_start);
            if next_sequence.is_none() || next_sequence.unwrap().start == playing_sequence.start {
                self.timeline.queue_sequence(next_start, playing_sequence.index);
            }
        }
    }

    // Get tick ranges of phrases that are playing in current cycle
    pub fn playing_phrases(&self, track_index: usize, tick_range: &TickRange) -> Vec<(TickRange, u32, u8)> {
        let playing_sequence = *self.timeline.playing_sequence(tick_range.start).unwrap();

        let sequence = &self.sequences[playing_sequence.index];
        let sequence_stop = playing_sequence.start + sequence.length(&self.tracks);

        let phrase_playing_at_start = sequence.get_phrase(track_index);

        let mut playing_phrases = vec![];

        if tick_range.contains(sequence_stop) {
            if let (Some(index), true) = (phrase_playing_at_start, tick_range.start < sequence_stop) {
                // Add from start to sequence_stop
                playing_phrases.push((TickRange::new(tick_range.start, sequence_stop), playing_sequence.start, index));
            }

            let next_sequence = self.timeline.playing_sequence(sequence_stop).unwrap();
            if let Some(index) = self.sequences[next_sequence.index].get_phrase(track_index) {
                // Only queue more of this when nothing is queued
                //TODO - Why did we use sequence_stop?
                playing_phrases.push((TickRange::new(sequence_stop, tick_range.stop), sequence_stop, index));
                //playing_phrases.push((TickRange::new(sequence_stop, tick_range.stop), playing_sequence.start, index));
            }
        } else {
            if let Some(index) = phrase_playing_at_start {
                playing_phrases.push((*tick_range, playing_sequence.start, index))
            }
        }

        playing_phrases
    }

    pub fn playing_patterns(&self, tick_range: &TickRange, track_index: usize, phrase_index: u8, sequence_start: u32) -> Vec<(u8, u32, TickRange, u32, u32)> {
        let track = &self.tracks[track_index];
        let phrase = track.phrase(phrase_index);

        // Get range relative to sequence
        let sequence_range = TickRange::new(tick_range.start - sequence_start, tick_range.stop - sequence_start);
        let phrase_ranges = phrase.looping_ranges(&sequence_range);

        phrase_ranges.into_iter()
            .flat_map(move |(phrase_range, phrase_offset)| {
                phrase.pattern_events.iter()
                    // Only pattern events that stop
                    .filter(|pattern_event| pattern_event.stop().is_some())
                    // Only pattern events that fall within relative phrase cycle
                    // Looping ranges are 2 ranges, start & end. Get absolute ranges and their
                    // corresponding offset in the pattern
                    .flat_map(move |pattern_event| {
                        pattern_event.absolute_tick_ranges(phrase.length()).into_iter()
                            .filter(move |(pattern_event_range, _)| pattern_event_range.overlaps(&phrase_range))
                            .map(move |(pattern_event_range, pattern_event_offset)| {
                                let pattern_event_length = pattern_event.length(phrase.length());
                                let absolute_offset = phrase_offset + sequence_start;

                                // Get range of pattern_event_range that falls within phrase_range
                                let absolute_start = if pattern_event_range.contains(phrase_range.start) { phrase_range.start } else { pattern_event_range.start };
                                let absolute_stop = if pattern_event_range.contains(phrase_range.stop) { phrase_range.stop } else { pattern_event_range.stop };

                                // Get relative range of pattern that should be played
                                let relative_range = TickRange::new(
                                    absolute_start - pattern_event_range.start + pattern_event_offset,
                                    absolute_stop - pattern_event_range.start + pattern_event_offset
                                );

                                (pattern_event.pattern, absolute_start, relative_range, pattern_event_length, absolute_offset)
                            })
                    })
            })
            .collect()
    }

    // TODO - Direct queueing
    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        if ! cycle.is_rolling {
            return
        }

        for track_index in 0 .. self.tracks.len() {
            let playing_phrases = self.playing_phrases(track_index, &cycle.tick_range);

            //let mut starting_notes = vec![];
            let notes: Vec<PlayingNoteEvent> = playing_phrases.into_iter()
                .flat_map(|(tick_range, sequence_start, phrase_index)| {
                    // TODO - Make the switch to first getting pattern events, then converting
                    // those to notes
                    self.playing_patterns(&tick_range, track_index, phrase_index, sequence_start).into_iter()
                        .flat_map(|(pattern_index, absolute_start, relative_range, pattern_event_length, absolute_offset)| {
                            let pattern = self.tracks[track_index].pattern(pattern_index);

                            // Get pattern based starting notes, and add offset based on phrase
                            // iteration & sequence start
                            pattern.starting_notes(absolute_start, relative_range, pattern_event_length).into_iter()
                                .map(move |mut playing_note| {
                                    playing_note.start += absolute_offset;
                                    playing_note.stop += absolute_offset;
                                    playing_note
                                })
                        })
                })
                .collect();

            self.tracks[track_index].output_midi(cycle, notes);
        }
    }

    // One of the control knobs on the APC was turned
    /*
    pub fn knob_turned(&mut self, time: u32, knob: u8, value: u8) {
        // Get the channel & knob that APC knob should send out of, first 8 channels are for
        // tracks, next 2 are used for sequences (sequence channels will be used for bus
        // effects)
        let (out_channel, out_knob) = match self.overview {
            OverView::Track => {
                let track_knob = self.track().set_knob_value(knob, value);
                let track = self.track_index() as u8;
                let offset = (track % 2) * 64;
                // 2 tracks per channel
                (track / 2, track_knob + offset)
            },
            OverView::Sequence => {
                let sequence_knob = self.sequence().set_knob_value(knob, value);
                // Sequence channels are after the tracks channels, therefore +
                // tracks.len / 2
                (self.sequence / 2 + self.tracks.len() as u8 / 2, sequence_knob + (self.sequence % 2) * 64)
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

        // Mulitple sequences and tracks live in same channels
        if knob >= 64 {
            knob_collection = knob_collection + 1;
            knob = knob - 64;
        }

        let knob_group = knob / 16;
        let apc_knob = knob % 16;

        // Pass change to correct knob group container
        let changed_knob = if knob_collection < 16 {
            self.tracks[knob_collection as usize].knob_value_changed(knob, message.bytes[2])
                // Knob was changed, see if track & knob_group are currently shown, if it is,
                // update knob on APC
                .and_then(|_| {
                    // Check if changed virtual knob is visible at the moment
                    if let OverView::Track = self.overview {
                        if self.track_index() as u8 == knob_collection && self.track().knob_group == knob_group {
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
                        // Sequence knob collections are placed after track groups
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

    pub fn recording_key_played(&mut self, track: u8, raw_channel: u8, cycle: &Cycle, message: jack::RawMidi) -> TimedMessage {
        // We're subtracting 9 as drumpads of my keyboard are outputting on channel 9
        let mut track_channel = raw_channel + track;

        if cycle.is_rolling {
            // Could be this is a note down meant for previously selected track
            let target: &mut Track = if self.tracks[track as usize].recorded_messages.len() == 0 && raw_channel == 0x80 {
                // Send message to track that has recorded messages left
                let (index, track) = self.tracks.iter_mut().enumerate()
                    .find(|(_, track)| track.recorded_messages.len() > 0)
                    .unwrap();

                track_channel = raw_channel + index as u8;
                track
            } else {
                // It was meant for current track instead
                &mut self.tracks[track as usize]
            };

            // Only record when cycle is rolling
            let cycle_length = cycle.end - cycle.start;
            // Message was recorded in previous frame
            let message_time = (cycle.start - cycle_length) + message.time;
            target.record_message(message_time, raw_channel, message.bytes[1], message.bytes[2], self.is_quantizing);
        }

        // Always play the note
        TimedMessage::new(message.time, Message::Note([track_channel, message.bytes[1], message.bytes[2]]))
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

    */
}
