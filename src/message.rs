
use std::cmp::Ordering;
 
#[derive(Debug, Eq, PartialEq)]
pub enum Message {
    Introduction([u8; 12]),
    Inquiry([u8; 6]),
    Note([u8; 3]),
}

#[derive(Debug, Eq)]
pub struct TimedMessage {
    pub time: u32,
    pub message: Message,
}

impl TimedMessage {
    pub fn new(time: u32, message: Message) -> Self {
        TimedMessage { time, message, }
    }

    pub fn to_raw_midi(&self) -> jack::RawMidi {
        match &self.message {
            Message::Introduction(bytes) =>                                            
                jack::RawMidi{ time: self.time, bytes: bytes},
            Message::Inquiry(bytes) =>                                                 
                jack::RawMidi{ time: self.time, bytes: bytes},
            Message::Note(bytes) =>                                                    
                jack::RawMidi{ time: self.time, bytes: bytes},
        }
    }
}

impl Ord for TimedMessage {
    fn cmp(&self, other: &TimedMessage) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for TimedMessage {
    fn partial_cmp(&self, other: &TimedMessage) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TimedMessage {
    fn eq(&self, other: &TimedMessage) -> bool {
        self.time == other.time
    }
}
