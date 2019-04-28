
use std::cmp::Ordering;
use super::message::{Message, MessageData};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Note {
    // Ticks in pattern that note should be played
    pub tick: u32,
    pub length: u32,

    pub key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    pub fn new(tick: u32, length: u32, key: u8, velocity: u8) -> Self {
        Note { tick, length, key, velocity, }
    }

    pub fn note_on(&self, frames: u32, channel: u8) -> Message {
        Message::new(frames, MessageData::Note([0x90 + channel, self.key, self.velocity]))
    }
    
    pub fn note_off(&self, frames: u32, channel: u8) -> Message {
        Message::new(frames, MessageData::Note([0x80 + channel, self.key, self.velocity]))
    }
}

#[derive(Debug, Eq)]
pub struct NoteOff {
    pub note: Note,
    pub tick: u32,
}

impl NoteOff {
    pub fn new(note: Note, tick: u32) -> Self {
        NoteOff { note, tick }
    }
}

impl Ord for NoteOff {
    fn cmp(&self, other: &NoteOff) -> Ordering {
        self.note.key.cmp(&other.note.key)
    }
}

impl PartialOrd for NoteOff {
    fn partial_cmp(&self, other: &NoteOff) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NoteOff {
    fn eq(&self, other: &NoteOff) -> bool {
        self.note.key == other.note.key
    }
}
