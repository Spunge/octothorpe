
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
    // Sets notes in patterns, or playing patterns in phrases, or playing phrases in timeline
    Grid(u8, u8),
    // Selects patterns & phrases
    Side(u8),
    // Indicates current playhead position
    Indicator(u8),
    // Indicates selected channel
    Channel(u8),
    // Indicates pattern, timeline or phrase length
    Green(u8),
    // Indicates pattern, timeline or phrase zoom
    Blue(u8),
    // Arms channels in sequence view, alters channel quantization in other views
    Red(u8),
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
pub enum InputEventType {
    KnobEvent(KnobType, u8),
    ButtonEvent(ButtonType, ButtonState),
    FaderEvent(FaderType, u8),
}

#[derive(Debug)]
pub struct InputEvent {
    time: u64,
    event_type: InputEventType,
}

impl InputEvent {
    pub fn new(time: u64, event_type: InputEventType) -> Self {
        Self { time, event_type }
    }
}

pub struct Grid { 
    pub width: u8,
    pub height: u8,
}

pub trait DeviceType {
    //fn process_InputEvent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
    //fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
    fn port_name(&self) -> &'static str;

    fn grid_mut(&mut self) -> Option<&mut Grid>;
    fn grid(&self) -> Option<&Grid>;

    fn process_rawmidi(&mut self, message: jack::RawMidi) -> Option<InputEventType>;

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
            0x34 => Some(ButtonType::Indicator(channel)),
            0x30 => Some(ButtonType::Red(channel)),
            0x31 => Some(ButtonType::Green(channel)),
            0x32 => Some(ButtonType::Blue(channel)),
            // APC40 specific stuff
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

impl DeviceType for APC {
    fn port_name(&self) -> &'static str { self.apc_type.port_name() }

    fn grid_mut(&mut self) -> Option<&mut Grid> { Some(&mut self.grid) }
    fn grid(&self) -> Option<&Grid> { Some(&self.grid) }

    fn process_rawmidi(&mut self, message: jack::RawMidi) -> Option<InputEventType> {
        let b = message.bytes;

        // 0x06 = inquiry e, 0x02 = inquiry response 0x47 = akai manufacturer, 0x73 = APC40, 0x7b = APC20
        if b[0] == 0xF0 && b[3] == 0x06 && b[4] == 0x02 && b[5] == 0x47 && (b[6] == 0x73 || b[6] == 0x7b) {
            println!("inquiry response received {:?} {:?}", b[13], b[6]);
            self.local_id = Some(b[13]);
            self.device_id = Some(b[6]);
        }

        // Get input event for rawmidi message
        match b[0] {
            // Button pressed or released
            0x80 ..= 0x9F => {
                let (channel, button_state) = if(b[0] > 0x8F) {
                    (b[0] - 0x90, ButtonState::Pressed)
                } else {
                    (b[0] - 0x80, ButtonState::Released)
                };

                self.byte_to_buttontype(b[1], channel)
                    .and_then(|button_type| {
                        Some(InputEventType::ButtonEvent(button_type, button_state))
                    })
            },
            // Fader or pot adjusted
            0xB0 ..= 0xB8 => {
                match b[1] {
                    0x30 ..= 0x37 | 0x10 ..= 0x17 => {
                        // APC effect knobs are ordered weird, reorder them from to 0..16
                        let modifier = if (0x30 ..= 0x37).contains(&b[1]) { 48 } else { 8 };
                        let index = b[1] - modifier;

                        Some(InputEventType::KnobEvent(KnobType::Control(index), b[2]))
                    },
                    0x7 => Some(InputEventType::FaderEvent(FaderType::Channel(b[0] - 0xB0), b[2])),
                    0xE => Some(InputEventType::FaderEvent(FaderType::Master, b[2])),
                    0xF => Some(InputEventType::FaderEvent(FaderType::CrossFade, b[2])),
                    0x2F => Some(InputEventType::KnobEvent(KnobType::Cue, b[2])),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    // Get midi messages that should be output
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
                // No ID know yet, inquire about device
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
}

pub struct Device {
    pub system_source: jack::Port<jack::Unowned>,
    pub system_sink: jack::Port<jack::Unowned>,

    pub input: jack::Port<jack::MidiIn>,
    pub output: jack::Port<jack::MidiOut>,

    pub device_type: Box<dyn DeviceType + Send>,
}

impl Device {
    // We expect that system always reports capture port first, so we can create hardware
    // representations when we see the capture port and add the playback port later
    pub fn new(
        client: &jack::Client,
        system_source: jack::Port<jack::Unowned>,
        system_sink: jack::Port<jack::Unowned>,
        device_type: impl DeviceType + Send + 'static
    ) -> Self {
        let port_name = device_type.port_name();

        // Get port names based on contoller type
        let mut input_port_name = port_name.to_owned();
        input_port_name.push_str("_in");
        let mut output_port_name = port_name.to_owned();
        output_port_name.push_str("_out");

        // Create devices jack midi ports
        let input = client.register_port(input_port_name.as_str(), jack::MidiIn::default()).unwrap();
        let output = client.register_port(output_port_name.as_str(), jack::MidiOut::default()).unwrap();

        // Connect this device
        match client.connect_ports(&system_source, &input) {
            Ok(_) => println!("{:?} connected", input_port_name),
            Err(e) => println!("{:?} could not be connected, {:?}", input_port_name, e),
        };
        match client.connect_ports(&output, &system_sink) {
            Ok(_) => println!("{:?} connected", output_port_name),
            Err(e) => println!("{:?} could not be connected, {:?}", output_port_name, e),
        };

        Self {
            system_source,
            system_sink,
            input,
            output,

            device_type: Box::new(device_type),
        }
    }

    // Get input events from this devices input midi port
    pub fn process_midi_input(&mut self, cycle: &ProcessCycle) -> Vec<InputEvent> {
        let mut events = vec![];
        for message in self.input.iter(cycle.scope) {
            if let Some(input_event_type) = self.device_type.process_rawmidi(message) {
                events.push(InputEvent::new(cycle.frame_to_time(message.time), input_event_type))
            }
        }

        events
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        let mut writer = self.output.writer(cycle.scope);
        let messages = self.device_type.get_midi_messages(cycle);

        for message in messages {
            writer.write(&message.to_jack_rawmidi());
        }
    }
}
