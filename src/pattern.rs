
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

pub struct Pattern {
    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
    pub base_note: u8,
    pub is_recording: bool,
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
        }
    }

    pub fn new(channel: u8) -> Self {
        Pattern::create(channel, vec![])
    }

    pub fn default(channel: u8) -> Self {
        let notes = vec![
            //Note::new(channel, TimebaseHandler::beats_to_ticks(0.0), TimebaseHandler::beats_to_ticks(2.0), 45, 127, 127),
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
        self.notes = vec![];
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

    // Toggle led range should support removing parts of the led grid
    pub fn toggle_led_range(&mut self, x: Range<u8>, y: u8, velocity_on: u8, velocity_off: u8) {
        let start = self.playable.ticks_offset() + self.playable.ticks_per_led() * x.start as u32;
        let end = self.playable.ticks_offset() + self.playable.ticks_per_led() * (x.end + 1) as u32;

        let key = self.base_note - y;

        // Check if we only need to remove notes
        if let Some(note) = self.notes.iter()
            .find(|note| {
                  let note_clicked = note.start == start && note.key == key;

                  let is_one_led = note.end == note.start + self.playable.ticks_per_led();

                  note_clicked && (! is_one_led || note.end == end)
            }) 
        {
            // Remove colliding notes
            self.notes.retain(|note| {
                note.end <= start || note.start >= end || note.key != key
            });
        } else {
            self.toggle_note(start, end, key, velocity_on, velocity_off)
        }
    }

    // Toggle note should draw note onto pattern grid so keyboard logic can use this to
    pub fn toggle_note(&mut self, mut start: u32, mut end: u32, key: u8, velocity_on: u8, velocity_off: u8) {
        start = start % self.playable.length;
        end = if end == self.playable.length { self.playable.length } else { end % self.playable.length };

        // Center view on notes we are recording
        if key > self.base_note {
            self.base_note = key;
        } 
        if key < self.base_note - 4 {
            self.base_note = key + 4;
        }

        if end < start {
            end = end + self.playable.length;
        }

        // Shorten previous note when a note is played
        for note in &mut self.notes {
            if note.start < start && note.end > start && note.key == key {
                note.end = start;
            }
            if note.start < end && note.end > end && note.key == key {
                note.start = end;
            }
        }

        // Keep the notes that dont collide with current note
        self.notes.retain(|note| {
            let remove_note = note.start >= start && note.end <= end && note.key == key;

            ! remove_note
        });

        self.notes.push(Note::new(self.channel, start, end, key, velocity_on, velocity_off));
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

