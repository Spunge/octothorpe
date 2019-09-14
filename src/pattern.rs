
use std::ops::Range;

use super::note::Note;
use super::playable::Playable;
use super::cycle::Cycle;
use super::handlers::TimebaseHandler;

#[derive(Debug, Clone)]
pub struct PlayedPattern {
    // Index in instruments patterns array
    pub index: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

pub struct PlayingPattern {
    pub instrument: usize,
    pub pattern: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

struct RecordedMessage {
    cycle_start: u32,
    channel: u8,
    key: u8,
    velocity: u8,
}

pub struct Pattern {
    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
    pub base_note: u8,
    pub is_recording: bool,
    recorded_messages: Vec<RecordedMessage>,
}

impl Pattern {
    const BASE_NOTE: u8 = 49;

    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            playable: Playable::new(TimebaseHandler::bars_to_ticks(1), TimebaseHandler::bars_to_ticks(1), 1, 5),
            channel,
            notes,
            // TODO - Use scales for this
            // Put a4 in center of grid
            base_note: Self::BASE_NOTE,
            is_recording: false,
            recorded_messages: vec![],
        }
    }

    pub fn new(channel: u8) -> Self {
        Pattern::create(channel, vec![])
    }

    pub fn default(channel: u8) -> Self {
        let notes = vec![
            Note::new(channel, TimebaseHandler::beats_to_ticks(0.0), TimebaseHandler::beats_to_ticks(2.0), 45, 127, 127),
            //Note::new(channel, TimebaseHandler::beats_to_ticks(1.0), TimebaseHandler::beats_to_ticks(1.5), 45, 127, 127),
            //Note::new(channel, TimebaseHandler::beats_to_ticks(2.0), TimebaseHandler::beats_to_ticks(2.5), 45, 127, 127),
            //Note::new(channel, TimebaseHandler::beats_to_ticks(3.0), TimebaseHandler::beats_to_ticks(3.5), 45, 127, 127),
        ];
        Pattern::create(channel, notes)
    }
    
    pub fn led_states(&mut self) -> Vec<(i32, i32, u8)> {
        let coords = self.notes.iter()
            // start, end, y
            .map(|note| (note.start, note.end, self.base_note as i32 - note.key as i32))
            .collect();

        self.playable.led_states(coords)
    }

    pub fn reset(&mut self) {
        self.base_note = Self::BASE_NOTE;
    }

    pub fn change_base_note(&mut self, delta: i32) {
        let base_note = self.base_note as i32 + delta;

        // 21 is A0
        if base_note >= 25 && base_note <= 127 {
            self.base_note = base_note as u8;
        }
    }

    // Start recording notes from input into pattern
    pub fn switch_recording_state(&mut self) {
        self.is_recording = ! self.is_recording;
    }

    pub fn record_message(&mut self, cycle_start: u32, message: jack::RawMidi) {
        // TODO - if message is note on, push it into recorded messages array
        // TODO - If message is note off, look in recorded messages for last note on and create a note
        
        println!("{:?}", cycle_start);
        println!("Key played: note_{:?} on channel {:?} played with velocity: {:?}", message.bytes[1], message.bytes[0], message.bytes[2]);
    }

    pub fn toggle_led_range(&mut self, x: Range<u8>, y: u8, velocity_on: u8, velocity_off: u8) {
        let start = self.playable.ticks_offset() + self.playable.ticks_per_led() * x.start as u32;
        let end = self.playable.ticks_offset() + self.playable.ticks_per_led() * (x.end + 1) as u32;

        let key = self.base_note - y;

        self.toggle_note(start, end, key, velocity_on, velocity_off)
    }

    pub fn toggle_note(&mut self, start: u32, end: u32, key: u8, velocity_on: u8, velocity_off: u8) {
        let notes = self.notes.len();
        
        let mut toggle_on = true;

        // Shorten pattern when a button is clicked that falls in the range of the note
        for note in &mut self.notes {
            if note.start < start && note.end > start && note.key == key {
                note.end = start;
                toggle_on = false;
            }
        }

        // Keep the notes that dont collide with current note
        self.notes.retain(|note| {
            let keep_note = (note.start < start && note.end <= start || note.start >= end) || note.key != key;
            if ! keep_note && note.start == start && note.end == end {
                toggle_on = false;
            }
            if ! keep_note && note.start == start && note.end > end {
                toggle_on = false;
            }
            keep_note
        });

        // No notes were removed, add new note, when note is longer as 1, the 1 note from the
        // previous keypress is removed, so ignore that
        if toggle_on {
            self.notes.push(Note::new(self.channel, start, end, key, velocity_on, velocity_off));
        }
    }

    pub fn playing_notes(&self, cycle: &Cycle, start: u32, end: u32) -> Vec<(u32, &Note)> {
         self.notes.iter()
            .filter_map(move |note| {
                let note_start = note.start + start;

                // Does note fall in cycle?
                if note_start >= cycle.start && note_start < cycle.end && note_start < end {
                    let delta_ticks = note_start - cycle.start;

                    Some((delta_ticks, note))
                } else {
                    None
                }
            })
            .collect()
    }
}

