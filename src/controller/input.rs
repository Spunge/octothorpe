
pub struct CueKnob {
    delta: i8,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ButtonType {
    Grid(u8, u8),
    Side(u8),
    Indicator(u8),
    Instrument(u8),
    Activator(u8),
    Solo(u8),
    Arm(u8),
    Sequence(u8),
    Shift,
    Quantization,
    Play,
    Stop,
    Up,
    Down,
    Right,
    Left,
    Unknown,
}

pub enum FaderType {
    Track(u8),
    Master,
}

pub enum KnobType {
    Effect { time: u32, index: u8},
    Cue,
}

pub enum Event {
    InquiryResponse(u8),
    ButtonPressed { time: u32, button_type: ButtonType },
    ButtonReleased { time: u32, button_type: ButtonType },
    KnobTurned { time: u32, value: u8, knob_type: KnobType },
    FaderMoved { time: u32, value: u8, fader_type: FaderType },
    Unknown,
}


impl ButtonType {
    fn new(channel: u8, note: u8) -> Self {
        match note {
            0x5B => ButtonType::Play,
            0x5C => ButtonType::Stop,
            0x33 => ButtonType::Instrument(channel),
            0x3F => ButtonType::Quantization,
            0x57 ..= 0x5A => ButtonType::Sequence(note - 0x57),
            // Side grid
            0x52 ..= 0x56 => ButtonType::Side(note - 0x52),
            0x51 => ButtonType::Shift,
            // Grid should add notes & add phrases
            0x35 ..= 0x39 => ButtonType::Grid(channel, note - 0x35),
            0x5E => ButtonType::Up,
            0x5F => ButtonType::Down,
            0x60 => ButtonType::Right,
            0x61 => ButtonType::Left,
            0x62 => ButtonType::Shift,
            0x30 => ButtonType::Arm(channel),
            0x31 => ButtonType::Solo(channel),
            0x32 => ButtonType::Activator(channel),
            _ => ButtonType::Unknown,
        }
    }
}

impl Event {
    pub fn new(time: u32, bytes: &[u8]) -> Self {
        match bytes[0] {
            0xF0 => {
                // 0x06 = inquiry e, 0x02 = inquiry response 0x47 = akai manufacturer, 0x73 = APC40, 0x7b = APC20
                if bytes[3] == 0x06 && bytes[4] == 0x02 && bytes[5] == 0x47 && (bytes[6] == 0x73 || bytes[6] == 0x7b) {
                    Self::InquiryResponse(bytes[13])
                } else {
                    Self::Unknown
                }
            },
            0x90 ..= 0x9F => Self::ButtonPressed { time, button_type: ButtonType::new(bytes[0] - 0x90, bytes[1]) },
            0x80 ..= 0x8F => Self::ButtonReleased { time, button_type: ButtonType::new(bytes[0] - 0x80, bytes[1]) },
            0xB0 ..= 0xB8 => {
                match bytes[1] {
                    0x30 ..= 0x37 | 0x10 ..= 0x17 => {
                        // APC effect knobs are ordered weird, reorder them from to 0..16
                        let modifier = if (0x30 ..= 0x37).contains(&bytes[1]) { 48 } else { 8 };
                        let index = bytes[1] - modifier;

                        Self::KnobTurned { time, value: bytes[2], knob_type: KnobType::Effect { time, index } }
                    },
                    0x7 => Self::FaderMoved { time, value: bytes[2], fader_type: FaderType::Track(bytes[0] - 0xB0) },
                    0xE => Self::FaderMoved { time, value: bytes[2], fader_type: FaderType::Master },
                    0x2F => Self::KnobTurned { time, value: bytes[2], knob_type: KnobType::Cue },
                    _ => Self::Unknown,
                }
            },
            _ => Self::Unknown,
        }
    }
}

/*
 * Struct that will decrease cueknob rotation speed a bit
 */
impl CueKnob {
    const CUE_KNOB_DELTA_PER_BUTTON: i8 = 8;

    pub fn new() -> Self { CueKnob { delta: 0 } }

    pub fn process_turn(&mut self, value: u8) -> i8 {
        // Transform 0->up / 128->down to -delta / +delta
        let delta = (value as i8).rotate_left(1) / 2;

        self.delta = self.delta + delta;

        let steps = self.delta / Self::CUE_KNOB_DELTA_PER_BUTTON;
        let remainder = self.delta % Self::CUE_KNOB_DELTA_PER_BUTTON;

        if steps != 0 {
            self.delta = remainder;
        }

        steps
    }
}

