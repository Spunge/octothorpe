
use std::cmp::Ordering;
use super::message::Message;

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

    pub fn note_off(&self, tick: u32) -> NoteOff {
        NoteOff::new(tick + self.length, self.key, self.velocity)
    }

    pub fn message(&self, channel: u8) -> Message {
        Message::Note([0x90 + channel, self.key, self.velocity])
    }
}

#[derive(Debug, Eq)]
pub struct NoteOff {
    pub tick: u32,
    pub key: u8,
    velocity: u8,
}

impl NoteOff {
    pub fn new(tick: u32, key: u8, velocity: u8) -> Self {
        NoteOff { tick, key, velocity }
    }

    pub fn message(&self, channel: u8) -> Message {
        Message::Note([0x80 + channel, self.key, self.velocity])
    }
}

impl Ord for NoteOff {
    fn cmp(&self, other: &NoteOff) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for NoteOff {
    fn partial_cmp(&self, other: &NoteOff) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NoteOff {
    fn eq(&self, other: &NoteOff) -> bool {
        self.key == other.key
    }
}
