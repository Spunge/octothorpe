
use std::cmp::Ordering;
 
#[derive(Debug, Eq, PartialEq)]
pub enum MessageData {
    Introduction([u8; 12]),
    Inquiry([u8; 6]),
    Note([u8; 3]),
}

#[derive(Debug, Eq)]
pub struct Message {
    pub time: u32,
    data: MessageData,
}

impl Message {
    pub fn new(time: u32, data: MessageData) -> Self {
        Message { time, data, }
    }

    pub fn to_raw_midi(&self) -> jack::RawMidi {
        match &self.data {
            MessageData::Introduction(bytes) =>                                            
                jack::RawMidi{ time: self.time, bytes: bytes},
            MessageData::Inquiry(bytes) =>                                                 
                jack::RawMidi{ time: self.time, bytes: bytes},
            MessageData::Note(bytes) =>                                                    
                jack::RawMidi{ time: self.time, bytes: bytes},
        }
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Message) -> Ordering {
        self.time.cmp(&other.time)
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Message) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Message) -> bool {
        self.time == other.time
    }
}
