
use super::TickRange;
use super::cycle::*;
use super::channel::Channel;
use super::sequence::Sequence;
use super::loopable::*;
use super::events::*;

pub struct Sequencer {
    pub channels: [Channel; 16],
    pub sequences: [Sequence; 5],

    pub sequence_playing: usize,
    pub sequence_queued: Option<usize>,
    pub last_sequence_started: u32,
}

impl Sequencer {
    pub fn new(client: &jack::Client) -> Self {
        // Build channels array, shame there's no way to do this elegantly without a macro as far as i can tell
        let channels = [
            Channel::new(client, 0),
            Channel::new(client, 1),
            Channel::new(client, 2),
            Channel::new(client, 3),
            Channel::new(client, 4),
            Channel::new(client, 5),
            Channel::new(client, 6),
            Channel::new(client, 7),
            Channel::new(client, 8),
            Channel::new(client, 9),
            Channel::new(client, 10),
            Channel::new(client, 11),
            Channel::new(client, 12),
            Channel::new(client, 13),
            Channel::new(client, 14),
            Channel::new(client, 15),
        ];

        // Build sequence we can trigger
        let sequences = [
            Sequence::new(0),
            Sequence::new(1),
            Sequence::new(2),
            Sequence::new(3),
            Sequence::new(4),
        ];

        Sequencer {
            channels,
            sequences,

            sequence_playing: 0,
            sequence_queued: None,
            last_sequence_started: 0,
        }
    }

    pub fn channel_mut(&mut self, index: usize) -> &mut Channel {
        &mut self.channels[index]
    }

    pub fn channel(&self, index: usize) -> &Channel {
        &self.channels[index]
    }

    pub fn get_sequence(&mut self, index: usize) -> &mut Sequence {
        &mut self.sequences[index]
    }

    pub fn start(&mut self, cycle: &ProcessCycle) {
        // Start playing notes, as it could be we halted mid channel
        self.channels.iter_mut().for_each(|channel| {
            channel.start_playing_notes(cycle);
        });

        cycle.client.transport_start();
    }

    pub fn stop(&mut self, cycle: &ProcessCycle) {
        cycle.client.transport_stop();

        // Output start of playing notes, as it could be we're starting mid channel
        self.channels.iter_mut().for_each(|channel| {
            channel.stop_playing_notes(cycle);
        });
    }

    pub fn reset(&mut self, cycle: &ProcessCycle) {
        // Reset position back to 0
        cycle.client.transport_reposition(jack::Position::default());

        // Clear playing notes
        self.channels.iter_mut().for_each(|channel| {
            channel.clear_playing_notes();
        });
    }

    pub fn reset_timeline(&mut self) {
        self.channels.iter_mut().for_each(|channel| {
            channel.timeline.clear_events();
        });
    }

    /*
     * Add playing phrases in sequence to respective channel timelines
     */
    pub fn play_sequence(&mut self, start: u32, sequence_index: usize) {
        let sequence = &self.sequences[sequence_index];
        let sequence_length = sequence.length(&self.channels);
        let stop = start + sequence_length;

        let active_phrases: Vec<(usize, u8)> = sequence.phrases().iter().enumerate()
            .filter(|(_, phrase_option)| phrase_option.is_some())
            .map(|(channel_index, phrase_option)| (channel_index, phrase_option.unwrap()))
            .collect();
        
        for (channel_index, phrase_index) in active_phrases {
            let mut phrase_start = start;
            let phrase_length = self.channel(channel_index).phrase(phrase_index).length();

            // When phrase is smaller than sequence, queue multiple smaller events
            while phrase_start < stop {
                let phrase_stop = if phrase_start + phrase_length > stop { stop } else { phrase_start + phrase_length };

                let event = LoopablePhraseEvent::new(phrase_start, phrase_stop, phrase_index);
                self.channel_mut(channel_index).timeline.add_complete_event(event);

                phrase_start += phrase_length;
            }
            
        }
    }

    // Get tick at which timeline stops
    pub fn get_timeline_end(&self) -> u32 {
        self.channels.iter()
            .map(|channel| channel.timeline.get_last_stop())
            .max()
            .unwrap()
    }

    pub fn autoqueue_next_sequence(&mut self, cycle: &ProcessCycle) {
        let timeline_end = self.get_timeline_end();

        if cycle.tick_range.contains(timeline_end) {
            if let Some(index) = self.sequence_queued {
                self.sequence_queued = None;
                self.sequence_playing = index;
            };

            self.play_sequence(timeline_end, self.sequence_playing);
        }
    }

    // Get tick ranges of phrases that are playing in current cycle
    pub fn playing_phrases(&self, channel_index: usize, tick_range: &TickRange) -> Vec<(TickRange, u32, u8)> {
        // Get phrase events that fall in tick_range
        self.channel(channel_index).timeline.events().iter()
            .filter(|event| event.stop().is_some())
            .filter(|event| tick_range.overlaps(&TickRange::new(event.start(), event.stop().unwrap())))
            // Only play start or only play end when they fall within tick_range
            .map(|event| {
                if tick_range.contains(event.stop().unwrap()) {
                    (TickRange::new(tick_range.start, event.stop().unwrap()), event.start(), event.phrase)
                } else if tick_range.contains(event.start()) {
                    (TickRange::new(event.start(), tick_range.stop), event.start(), event.phrase)
                } else {
                    (*tick_range, event.start(), event.phrase)
                }
            })
            .collect()
    }

    // Get tick ranges of patterns that are playing in tick_range
    pub fn playing_patterns(&self, tick_range: &TickRange, channel_index: usize, phrase_index: u8, sequence_start: u32) -> Vec<(u8, u32, TickRange, u32, u32)> {
        let channel = &self.channels[channel_index];
        let phrase = channel.phrase(phrase_index);

        // Get range relative to sequence
        let sequence_range = TickRange::new(tick_range.start - sequence_start, tick_range.stop - sequence_start);
        let phrase_ranges = phrase.looping_ranges(&sequence_range);

        phrase_ranges.into_iter()
            .flat_map(move |(phrase_range, phrase_offset)| {
                phrase.pattern_events.iter()
                    // Only pattern events that stop
                    .filter(|pattern_event| pattern_event.stop().is_some())
                    .filter(move |pattern_event| pattern_event.start() < phrase.length())
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

        for channel_index in 0 .. self.channels.len() {
            let playing_phrases = self.playing_phrases(channel_index, &cycle.tick_range);

            //let mut starting_notes = vec![];
            let notes: Vec<PlayingNoteEvent> = playing_phrases.into_iter()
                .flat_map(|(tick_range, sequence_start, phrase_index)| {
                    // TODO - Make the switch to first getting pattern events, then converting
                    // those to notes
                    self.playing_patterns(&tick_range, channel_index, phrase_index, sequence_start).into_iter()
                        .flat_map(|(pattern_index, absolute_start, relative_range, pattern_event_length, absolute_offset)| {
                            let pattern = self.channels[channel_index].pattern(pattern_index);

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

            self.channels[channel_index].output_midi(cycle, notes);
        }
    }
}
