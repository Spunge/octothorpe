
pub struct CueKnob {
    delta: i8,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ButtonType {
    Grid(u8, u8),
    Side(u8),
    Indicator(u8),
    Track(u8),
    Activator(u8),
    Solo(u8),
    Arm(u8),
    Shift,
    Quantization,
    Play,
    Stop,
    Up,
    Down,
    Right,
    Left,
    Master,
    Unknown,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FaderType {
    Track(u8),
    CrossFade,
    Master,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum KnobType {
    Effect(u8),
    Cue,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum InputEventType {
    InquiryResponse(u8, u8),
    ButtonPressed(ButtonType),
    ButtonReleased(ButtonType),
    KnobTurned { value: u8, knob_type: KnobType },
    FaderMoved { value: u8, fader_type: FaderType },
    Unknown,
}

pub struct InputEvent {
    pub time: u32,
    pub event_type: InputEventType,
}

impl ButtonType {
    fn new(channel: u8, note: u8) -> Self {
        match note {
            0x5B => ButtonType::Play,
            0x5C => ButtonType::Stop,
            0x33 => ButtonType::Track(channel),
            0x3F => ButtonType::Quantization,
            // These used to be sequence buttons, but will now be more control groups for plugin parameters
            //0x57 ..= 0x5A => ButtonType::Sequence(note - 0x57),
            // Side grid is turned upside down as we draw the phrases upside down as we draw notes
            // updside down due to lower midi nodes having lower numbers, therefore the 4 -
            0x52 ..= 0x56 => ButtonType::Side(4 - (note - 0x52)),
            0x51 => ButtonType::Shift,
            0x50 => ButtonType::Master,
            // Grid should add notes & add phrases
            0x35 ..= 0x39 => ButtonType::Grid(channel, 4 - (note - 0x35)),
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

/*
 * Get input event type from sent bytes
 */
impl InputEventType {
    pub fn new(bytes: &[u8]) -> Self {
         match bytes[0] {
            0xF0 => {
                // 0x06 = inquiry e, 0x02 = inquiry response 0x47 = akai manufacturer, 0x73 = APC40, 0x7b = APC20
                if bytes[3] == 0x06 && bytes[4] == 0x02 && bytes[5] == 0x47 && (bytes[6] == 0x73 || bytes[6] == 0x7b) {
                    Self::InquiryResponse(bytes[13], bytes[6])
                } else {
                    Self::Unknown
                }
            },
            0x90 ..= 0x9F => Self::ButtonPressed(ButtonType::new(bytes[0] - 0x90, bytes[1])),
            0x80 ..= 0x8F => Self::ButtonReleased(ButtonType::new(bytes[0] - 0x80, bytes[1])),
            0xB0 ..= 0xB8 => {
                match bytes[1] {
                    0x30 ..= 0x37 | 0x10 ..= 0x17 => {
                        // APC effect knobs are ordered weird, reorder them from to 0..16
                        let modifier = if (0x30 ..= 0x37).contains(&bytes[1]) { 48 } else { 8 };
                        let index = bytes[1] - modifier;

                        Self::KnobTurned { value: bytes[2], knob_type: KnobType::Effect(index) }
                    },
                    0x7 => Self::FaderMoved { value: bytes[2], fader_type: FaderType::Track(bytes[0] - 0xB0) },
                    0xE => Self::FaderMoved { value: bytes[2], fader_type: FaderType::Master },
                    0xF => Self::FaderMoved { value: bytes[2], fader_type: FaderType::CrossFade },
                    0x2F => Self::KnobTurned { value: bytes[2], knob_type: KnobType::Cue },
                    _ => Self::Unknown,
                }
            },
            _ => Self::Unknown,
        }
    }
}

impl InputEvent {
    pub fn new(time: u32, bytes: &[u8]) -> Self {
        Self { time, event_type: InputEventType::new(bytes) }
    }

    pub fn is_cue_knob(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::KnobTurned { knob_type: KnobType::Cue, .. }) 
    }

    pub fn is_crossfader(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::FaderMoved { fader_type: FaderType::CrossFade, .. }) 
    }

    pub fn is_activator_button(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Activator(_)))
    }

    pub fn is_track_button(event_type: &InputEventType) -> bool {
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Track(_)))
    }

    pub fn is_solo_button(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Solo(_)))
    }

    pub fn is_grid_button(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Grid(_, _)))
    }

    pub fn is_right_button(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Right))
    }

    pub fn is_left_button(event_type: &InputEventType) -> bool { 
        matches!(event_type, InputEventType::ButtonPressed(ButtonType::Left))
    }
}

/*
 * Struct that will decrease cueknob rotation speed a bit
 */
impl CueKnob {
    const DELTA_PER_BUTTON: i8 = 6;

    pub fn new() -> Self { CueKnob { delta: 0 } }

    // TODO - Use time for this aswell, so that turning knob instantly moves grid
    pub fn process_turn(&mut self, value: u8, is_first_turn: bool) -> i8 {
        // Transform 0->up / 128->down to -delta / +delta
        let delta = (value as i8).rotate_left(1) / 2;

        // Reset on first turn and return 1 step
        if is_first_turn {
            self.delta = 0;

            if delta > 0 { 1 } else { -1 }
        } else {
            self.delta = self.delta + delta;

            let steps = self.delta / Self::DELTA_PER_BUTTON;
            let remainder = self.delta % Self::DELTA_PER_BUTTON;

            if steps != 0 {
                self.delta = remainder;
            }

            steps
        }
    }
}

