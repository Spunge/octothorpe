
#[derive(Debug)]
pub enum Message {
    Introduction(u32, [u8; 12]),
    Inquiry(u32, [u8; 6]),
    Note(u32, [u8; 3]),
}

impl Message {
    pub fn to_raw_midi(&self) -> jack::RawMidi {
        match self {
            Message::Introduction(time, bytes) =>                                            
                jack::RawMidi{ time: *time, bytes: bytes},
            Message::Inquiry(time, bytes) =>                                                 
                jack::RawMidi{ time: *time, bytes: bytes},
            Message::Note(time, bytes) =>                                                    
                jack::RawMidi{ time: *time, bytes: bytes},
        }
    }
}

