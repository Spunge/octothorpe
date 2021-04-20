
//pub mod apc40;
//pub mod apc20;

//pub use self::apc40::OLD_APC40;
//pub use self::apc20::OLD_APC20;

use crate::*;

#[derive(Debug)]
pub struct MidiMessage {
    frame: u32,
    bytes: Vec<u8>,
}

impl MidiMessage {
    pub fn new(frame: u32, bytes: Vec<u8>) -> Self {
        Self { frame, bytes }
    }

    pub fn to_jack_rawmidi(&self) -> jack::RawMidi {
        jack::RawMidi {
            time: self.frame,
            bytes: &self.bytes.as_slice(),
        }
    }
}

#[derive(Debug)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug)]
pub enum ButtonType {
    Grid(u8, u8),
    Side(u8),
    Indicator(u8),
    Channel(u8),
    Length(u8),
    Zoom(u8),
    //Arm(u8),
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

#[derive(Debug)]
pub enum FaderType {
    Channel(u8),
    CrossFade,
    Master,
}

#[derive(Debug)]
pub enum KnobType {
    Control(u8),
    Cue,
}

#[derive(Debug)]
pub enum ControllerEventType {
    KnobEvent(KnobType),
    ButtonEvent(ButtonType, ButtonState),
    FaderEvent(FaderType),
}

#[derive(Debug)]
pub struct ControllerEvent {
    time: u64,
    event_type: ControllerEventType,
}

impl ControllerEvent {
    pub fn new(time: u64, event_type: ControllerEventType) -> Self {
        Self { time, event_type }
    }
}

pub struct Grid { 
    pub width: u8,
    pub height: u8,
}

pub trait ControllerType {
    //fn process_ControllerEvent(&mut self, event: &ControllerEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
    //fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
    fn port_name(&self) -> &'static str;

    fn grid_mut(&mut self) -> Option<&mut Grid>;
    fn grid(&self) -> Option<&Grid>;

    fn process_rawmidi(&mut self, bytes: &[u8]) -> Option<ControllerEventType>;

    fn get_midi_messages(&mut self, cycle: &ProcessCycle) -> Vec<MidiMessage>;
}

pub struct APC {
    apc_type: Box<dyn APCType + Send>,
    introduced_at: u64,
    local_id: Option<u8>,
    device_id: Option<u8>,

    grid: Grid,
}

impl APC {
    pub fn new(apc_type: impl APCType + Send + 'static) -> Self {
        Self {
            apc_type: Box::new(apc_type),
            introduced_at: 0,
            local_id: None,
            device_id: None,

            grid: Grid { width: 8, height: 5 },
        }
    }

    fn byte_to_buttontype(&self, byte: u8, channel: u8) -> Option<ButtonType> {
        match byte {
            0x33 => Some(ButtonType::Channel(channel)),
            // These used to be sequence buttons, but will now be more control groups for plugin parameters
            //0x57 ..= 0x5A => ButtonType::Sequence(byte - 0x57),
            // Side grid is turned upside down as we draw the phrases upside down as we draw notes
            // updside down due to lower midi nodes having lower numbers, therefore the 4 -
            0x52 ..= 0x56 => Some(ButtonType::Side(4 - (byte - 0x52))),
            0x51 => Some(ButtonType::Shift),
            0x62 => Some(ButtonType::Shift),
            0x50 => Some(ButtonType::Master),
            // Grid should add notes & add phrases
            0x35 ..= 0x39 => Some(ButtonType::Grid(channel, 4 - (byte - 0x35))),
            //0x30 => ButtonType::Arm(channel),
            0x31 => Some(ButtonType::Zoom(channel)),
            0x32 => Some(ButtonType::Length(channel)),
            _ => self.apc_type.byte_to_buttontype(byte, channel),
        }
    }
}

impl ControllerType for APC {
    fn port_name(&self) -> &'static str { self.apc_type.port_name() }

    fn grid_mut(&mut self) -> Option<&mut Grid> { Some(&mut self.grid) }
    fn grid(&self) -> Option<&Grid> { Some(&self.grid) }

    fn process_rawmidi(&mut self, bytes: &[u8]) -> Option<ControllerEventType> {
        match bytes[0] {
            // Sysex message
            0xF0 => {
                // 0x06 = inquiry e, 0x02 = inquiry response 0x47 = akai manufacturer, 0x73 = APC40, 0x7b = APC20
                if bytes[3] == 0x06 && bytes[4] == 0x02 && bytes[5] == 0x47 && (bytes[6] == 0x73 || bytes[6] == 0x7b) {
                    println!("inquiry response received {:?} {:?}", bytes[13], bytes[6]);
                    self.local_id = Some(bytes[13]);
                    self.device_id = Some(bytes[6]);
                }
                None
            },
            // Button pressed or released
            0x80 ..= 0x9F => {
                let (channel, button_state) = if(bytes[0] > 0x8F) {
                    (bytes[0] - 0x90, ButtonState::Pressed)
                } else {
                    (bytes[0] - 0x80, ButtonState::Released)
                };

                self.byte_to_buttontype(bytes[1], channel)
                    .and_then(|button_type| {
                        Some(ControllerEventType::ButtonEvent(button_type, button_state))
                    })
            },
            /*
            0xB0 ..= 0xB8 => {
                match bytes[1] {
                    0x30 ..= 0x37 | 0x10 ..= 0x17 => {
                        // APC effect knobs are ordered weird, reorder them from to 0..16
                        let modifier = if (0x30 ..= 0x37).contains(&bytes[1]) { 48 } else { 8 };
                        let index = bytes[1] - modifier;

                        Self::KnobTurned { value: bytes[2], knob_type: KnobType::Control(index) }
                    },
                    0x7 => Self::FaderMoved { value: bytes[2], fader_type: FaderType::Channel(bytes[0] - 0xB0) },
                    0xE => Self::FaderMoved { value: bytes[2], fader_type: FaderType::Master },
                    0xF => Self::FaderMoved { value: bytes[2], fader_type: FaderType::CrossFade },
                    0x2F => Self::KnobTurned { value: bytes[2], knob_type: KnobType::Cue },
                    _ => Self::Unknown,
                }
            },
            */
            _ => None,
        }
    }

    fn get_midi_messages(&mut self, cycle: &ProcessCycle) -> Vec<MidiMessage> {
        let mut messages = vec![];

        if self.introduced_at == 0 {
            // First run, not introduces
            if let (Some(local_id), Some(device_id)) = (self.local_id, self.device_id) {
                // ID is known, introduce ourselves
                self.introduced_at = cycle.time_start;
                println!("writing introduction");
                messages.push(MidiMessage::new(0, vec![0xF0, 0x47, local_id, device_id, 0x60, 0x00, 0x04, 0x42, 0x00, 0x00, 0x00, 0xF7]))
            } else {
                // No ID know yet, inquire about controller
                println!("writing inquiry");
                messages.push(MidiMessage::new(0, vec![0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]));
            }
        } else {
            // We have been introduced, wait a bit for APC to switch to alternate mode
        }

        messages
    }
}

pub trait APCType {
    fn port_name(&self) -> &'static str;

    fn byte_to_buttontype(&self, byte: u8, channel: u8) -> Option<ButtonType>;
}

pub struct APC40 {
}

impl APC40 {
    pub fn new() -> Self {
        Self {}
    }
}

impl APCType for APC40 {
    fn port_name(&self) -> &'static str { "apc40" }

    fn byte_to_buttontype(&self, byte: u8, channel: u8) -> Option<ButtonType> {
        match byte {
            0x5B => Some(ButtonType::Play),
            0x5C => Some(ButtonType::Stop),
            0x3F => Some(ButtonType::Quantization),
            0x5E => Some(ButtonType::Up),
            0x5F => Some(ButtonType::Down),
            0x60 => Some(ButtonType::Right),
            0x61 => Some(ButtonType::Left),
            _ => None,
        }
    }
}

pub struct APC20 {
}

impl APC20 {
    pub fn new() -> Self {
        Self {}
    }
}

impl APCType for APC20 {
    fn port_name(&self) -> &'static str { "apc20" }

    fn byte_to_buttontype(&self, byte: u8, channel: u8) -> Option<ButtonType> {
        None
    }
}

pub struct Controller {
    pub system_source: jack::Port<jack::Unowned>,
    pub system_sink: jack::Port<jack::Unowned>,

    pub input: jack::Port<jack::MidiIn>,
    pub output: jack::Port<jack::MidiOut>,

    pub controller_type: Box<dyn ControllerType + Send>,
}

impl Controller {
    // We expect that system always reports capture port first, so we can create hardware
    // representations when we see the capture port and add the playback port later
    pub fn new(
        client: &jack::Client,
        system_source: jack::Port<jack::Unowned>,
        system_sink: jack::Port<jack::Unowned>,
        controller_type: impl ControllerType + Send + 'static
    ) -> Self {
        let port_name = controller_type.port_name();

        // Get port names based on contoller type
        let mut input_port_name = port_name.to_owned();
        input_port_name.push_str("_in");
        let mut output_port_name = port_name.to_owned();
        output_port_name.push_str("_out");

        // Create controllers jack midi ports
        let input = client.register_port(input_port_name.as_str(), jack::MidiIn::default()).unwrap();
        let output = client.register_port(output_port_name.as_str(), jack::MidiOut::default()).unwrap();

        // Connect this controller
        client.connect_ports(&system_source, &input);
        client.connect_ports(&output, &system_sink);

        Self {
            system_source,
            system_sink,
            input,
            output,

            controller_type: Box::new(controller_type),
        }
    }

    // Get input events from this controllers input midi port
    pub fn process_midi_input(&mut self, cycle: &ProcessCycle, octothorpe: &mut Octothorpe, others: &Vec<Controller>) {
        //println!("{:?}", self.controller_type.port_name());
        let names: Vec<&str> = others.iter().map(|controller| controller.controller_type.port_name()).collect();
        //println!("others {:?}", names);

        for message in self.input.iter(cycle.scope) {
            // Let controller type handle midi
            if let Some(event_type) = self.controller_type.process_rawmidi(message.bytes) {
                // If message is a recognized event type, make event
                let controller_event = ControllerEvent::new(cycle.time_at_frame(message.time), event_type);
                println!("{:?}", controller_event);
            }
        }
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle, octothorpe: &mut Octothorpe, others: &Vec<Controller>) {
        let mut writer = self.output.writer(cycle.scope);
        let messages = self.controller_type.get_midi_messages(cycle);

        for message in messages {
            writer.write(&message.to_jack_rawmidi());
        }
    }
}
