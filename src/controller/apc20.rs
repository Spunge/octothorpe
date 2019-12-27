
use super::*;

pub struct APC20 {
    memory: Memory,

    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    is_identified: bool,
    offset: u8,
}
