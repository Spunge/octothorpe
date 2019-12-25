
use super::controller::Controller;

pub struct APC40 {
    // Ports that connect to APCs
    input_port: jack::Port<jack::MidiIn>,
    output_port: MidiOut,

    pressed_keys: Vec<PressedKey>,
}
