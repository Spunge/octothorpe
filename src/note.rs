
use super::message::{Message, MessageData};

#[derive(Debug, Clone, Copy)]
pub struct Note {
    // Ticks in pattern that note should be played
    pub tick: u32,
    pub length: u32,

    pub key: u8,
    velocity: u8,
}

impl Note {
    // Create A4 quarter note
    pub fn new(tick: u32, length: u32, key: u8) -> Self {
        Note { tick, length, key: key, velocity: 127 }
    }

    pub fn note_on(&self, frames: u32) -> Message {
        Message::new(frames, MessageData::Note([0x90, self.key, self.velocity]))
    }
    
    pub fn note_off(&self, frames: u32) -> Message {
        Message::new(frames, MessageData::Note([0x80, self.key, self.velocity]))
    }
}

#[derive(Debug)]
pub struct NoteOff {
    pub note: Note,
    pub tick: u32,
}

impl NoteOff {
    pub fn new(note: Note, tick: u32) -> Self {
        NoteOff { note, tick }
    }
}

