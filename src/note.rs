
use std::cmp::Ordering;
use super::message::Message;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Note {
    // Ticks in pattern that note should be played
    pub start: u32,
    pub end: u32,
    pub key: u8,
    velocity: u8,
    channel: u8,
}

impl Note {
    // Create A4 quarter note
    pub fn new(channel: u8, start: u32, end: u32, key: u8, velocity: u8) -> Self {
        Note { channel, start, end, key, velocity, }
    }

    pub fn note_off(&self, absolute_tick: u32) -> NoteOff {
        NoteOff::new(self.channel, absolute_tick + (self.end - self.start), self.key, self.velocity)
    }

    pub fn message(&self) -> Message {
        Message::Note([0x90 + self.channel, self.key, self.velocity])
    }
}

#[derive(Debug, Eq)]
pub struct NoteOff {
    pub tick: u32,
    pub key: u8,
    velocity: u8,
    channel: u8,
}

impl NoteOff {
    pub fn new(channel: u8, tick: u32, key: u8, velocity: u8) -> Self {
        NoteOff { channel, tick, key, velocity }
    }

    pub fn message(&self) -> Message {
        Message::Note([0x80 + self.channel, self.key, self.velocity])
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
