
use std::cmp::Ordering;
use super::message::Message;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Note {
    // Ticks in pattern that note should be played
    pub start: u32,
    pub end: u32,
    pub key: u8,
    pub velocity: u8,
    pub channel: u8,
}

impl Note {
    // Create A4 quarter note
    pub fn new(channel: u8, start: u32, end: u32, key: u8, velocity: u8) -> Self {
        Note { channel, start, end, key, velocity, }
    }

    pub fn message(&self, modifier: u8) -> Message {
        Message::Note([modifier + self.channel, self.key, self.velocity])
    }
}
