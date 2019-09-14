
use super::message::Message;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Note {
    // Ticks in pattern that note should be played
    pub start: u32,
    pub end: u32,
    pub key: u8,
    pub velocity_on: u8,
    pub velocity_off: u8,
    pub channel: u8,
}

impl Note {
    // Create A4 quarter note
    pub fn new(channel: u8, start: u32, end: u32, key: u8, velocity_on: u8, velocity_off: u8) -> Self {
        Note { channel, start, end, key, velocity_on, velocity_off }
    }

    // Use key passed or own key
    pub fn on_message(&self, modifier: u8, key: Option<u8>, velocity: Option<u8>) -> Message {
        Message::Note([
            modifier + self.channel,
            key.or(Some(self.key)).unwrap(),
            velocity.or(Some(self.velocity_on)).unwrap(),
        ])
    }

    // Use key passed or own key
    pub fn off_message(&self, modifier: u8, key: Option<u8>, velocity: Option<u8>) -> Message {
        Message::Note([
            modifier + self.channel,
            key.or(Some(self.key)).unwrap(),
            velocity.or(Some(self.velocity_off)).unwrap(),
        ])
    }
}
