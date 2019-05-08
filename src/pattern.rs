
use std::ops::Range;

use super::{beats_to_ticks, bars_to_ticks};
use super::note::{Note, NoteOff};
use super::message::{TimedMessage, Message};
use super::playable::Playable;
use super::cycle::Cycle;

const BASE_NOTE: u8 = 73;

pub struct Pattern {
    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
    base_note: u8,
}

impl Pattern {
    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            playable: Playable::new(bars_to_ticks(1), bars_to_ticks(1), 1, 5),
            channel,
            notes,
            // TODO - Use scales for this
            // Put a4 in center of grid
            base_note: BASE_NOTE,
        }
    }

    pub fn new(channel: u8) -> Self {
        Pattern::create(channel, vec![])
    }

    pub fn default(channel: u8) -> Self {
        let notes = vec![
            Note::new(channel, beats_to_ticks(0.0), beats_to_ticks(0.5), 73, 127),
            Note::new(channel, beats_to_ticks(1.0), beats_to_ticks(1.5), 69, 127),
            Note::new(channel, beats_to_ticks(2.0), beats_to_ticks(2.5), 69, 127),
            Note::new(channel, beats_to_ticks(3.0), beats_to_ticks(3.5), 69, 127),
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
        self.notes = vec![];
        self.base_note = BASE_NOTE;
    }

    pub fn change_base_note(&mut self, delta: i32) {
        let base_note = self.base_note as i32 + delta;

        // 21 is A0
        if base_note >= 25 && base_note <= 127 {
            self.base_note = base_note as u8;
        }
    }

    pub fn toggle_note(&mut self, x: Range<u8>, y: u8) {
        let start = self.playable.ticks_offset() + self.playable.ticks_per_led() * x.start as u32;
        let end = self.playable.ticks_offset() + self.playable.ticks_per_led() * (x.end + 1) as u32;

        let key = self.base_note - y;
        // TODO Velocity

        let notes = self.notes.len();
        
        // Shorten pattern when a button is clicked that falls in the range of the note
        for note in &mut self.notes {
            if note.start < start && note.end > start && note.key == key {
                note.end = start;
            }
        }

        self.notes.retain(|note| {
            (note.start < start && note.end <= start || note.start >= end) || note.key != key
        });

        // No notes were removed, add new note, when note is longer as 1, the 1 note from the
        // previous keypress is removed, so ignore that
        if notes == self.notes.len() || x.start != x.end {
            self.notes.push(Note::new(self.channel, start, end, key, 127));
        }
    }

    fn playing_speedable_notes(&self, cycle: &Cycle, start: u32, end: u32, modifier: u32) -> Vec<(TimedMessage, NoteOff)> {
         self.notes.iter()
            .filter_map(move |note| {
                let note_start = note.start + start;

                // Does note fall in cycle?
                if note_start >= cycle.start && note_start < cycle.end && note_start < end {
                    let delta_ticks = note_start - cycle.start;
                    let delta_frames = (delta_ticks as f64 / cycle.ticks as f64 * cycle.frames as f64) as u32;

                    let message = TimedMessage::new(delta_frames, note.message());
                    let note_off = note.note_off(cycle.absolute_start + delta_ticks + ((note.end - note.start) / modifier));

                    Some((message, note_off))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn playing_indicators(&self, cycle: &Cycle, start: u32, end: u32) -> Vec<(TimedMessage, (u32, u8))> {
        self.playing_speedable_notes(cycle, start, end, 2).into_iter()
            // Overwrite note & velocity for indicator
            .map(|(mut message, noteoff)| {
                if let Message::Note(mut bytes) = message.message {
                    bytes[1] = 0x34;
                    bytes[2] = 0x01;
                    message.message = Message::Note(bytes);
                }

                (message, (noteoff.tick, noteoff.channel))
            })
            .collect()
    }

    pub fn playing_notes(&self, cycle: &Cycle, start: u32, end: u32) -> Vec<(TimedMessage, NoteOff)> {
        self.playing_speedable_notes(cycle, start, end, 1)
    }
}

