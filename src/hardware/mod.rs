
//pub mod apc40;
//pub mod apc20;

//pub use self::apc40::OLD_APC40;
//pub use self::apc20::OLD_APC20;

use std::collections::HashMap;
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

pub trait DeviceType {
    //fn process_InputEvent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
    //fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
    fn port_name(&self) -> &'static str;

    fn surface(&self, name: &str) -> Option<&Surface> { None }
    fn surface_mut(&mut self, name: &str) -> Option<&mut Surface> { None }

    fn process_midi_inputevent(&mut self, midi_input_event: MidiInputEvent, octothorpe: &mut Octothorpe);

    fn get_midi_messages(&mut self, cycle: &ProcessCycle) -> Vec<MidiMessage>;
}

#[derive(Debug)]
pub struct Surface {
    width: u8,
    height: u8,
    offset_x: u8,
    surface_type: SurfaceType,
}

impl Surface {
    pub fn new(width: u8, height: u8, surface_type: SurfaceType) -> Self {
        Self { width, height, offset_x: 0, surface_type }
    }

    pub fn set_offset_x(&mut self, offset_x: u8) {
        self.offset_x = offset_x;
    }

    pub fn process_event(&mut self, mut event: SurfaceEvent) {
        event.offset_x(self.offset_x);
        println!("{:?}", self);
        println!("Processed the following event:");
        println!("{:?}", event);
        println!("");
    }
}

#[derive(Debug)]
pub enum ControlType {
    Absolute,
    Relative,
}

#[derive(Debug)]
pub enum SurfaceType {
    Button(ButtonSurfaceType),
    Control(ControlType, ControlSurfaceType),
}

#[derive(Debug)]
pub struct SurfaceEvent {
    time: u64,
    position: Position,
    surface_event_type: SurfaceEventType,
}

impl SurfaceEvent {
    pub fn new(time: u64, position: Position, surface_event_type: SurfaceEventType) -> Self {
        Self { time, position, surface_event_type }
    }

    pub fn offset_x(&mut self, offset: u8) {
        self.position.x += offset;
    }
}

#[derive(Debug)]
pub enum SurfaceEventType {
    Button(ButtonState),
    Control(u8),
}

#[derive(Debug)]
pub enum ButtonSurfaceType {
    PatternLength,
    PatternZoom,
}

#[derive(Debug)]
pub enum ControlSurfaceType {
    Parameter,
    Volume,
    HorizontalPosition,
    VerticalPosition,
}

#[derive(Debug)]
pub struct Position {
    x: u8,
    y: u8,
}

impl Position {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y, }
    }
}

pub struct Dimensions {
    width: u8,
    height: u8,
}

pub struct MidiInputEvent<'a> {
    bytes: &'a [u8],
    time: u64,
}

pub struct APC {
    apc_type: APCType,
    introduced_at: u64,
    local_id: Option<u8>,
    device_id: Option<u8>,
    
    surfaces: HashMap<&'static str, Surface>,
}

impl APC {
    pub fn new(apc_type: APCType, other_devices: &Vec<Device>) -> Self {
        // Keep track of surfaces in hashmap, so we can map over keys
        let mut surfaces = HashMap::new();
        surfaces.insert(
            "pattern_length",
            Surface::new(8, 1, SurfaceType::Button(ButtonSurfaceType::PatternLength)),
        );
        surfaces.insert(
            "pattern_zoom",
            Surface::new(8, 1, SurfaceType::Button(ButtonSurfaceType::PatternZoom)),
        );

        // Offset every surface by adding width of corresponding surface of already existing devices
        for (key, surface) in surfaces.iter_mut() {
            let offset_x = other_devices.iter()
                .filter_map(|device| {
                    device.device_type.surface(key)
                        .and_then(|surface| Some(surface.width))
                })
                .reduce(|a, b| a + b)
                .or(Some(0))
                .unwrap();

            surface.set_offset_x(offset_x);
        }

         Self {
            apc_type,
            introduced_at: 0,
            local_id: None,
            device_id: None,
            surfaces,
        }
    }

    fn recognized_button(&mut self, channel: u8, byte: u8) -> Option<(&mut Surface, Position)> {
        match byte {
            //0x33 => Some(ButtonType::Channel(channel)),
            // These used to be sequence buttons, but will now be more control groups for plugin parameters
            //0x57 ..= 0x5A => ButtonType::Sequence(b[1]- 0x57),
            // Side grid is turned upside down as we draw the phrases upside down as we draw notes
            // updside down due to lower midi nodes having lower numbers, therefore the 4 -
            //0x52 ..= 0x56 => Some(ButtonType::Side(4 - (b[1]- 0x52))),
            //0x51 => Some(ButtonType::View),
            //0x62 => Some(ButtonType::Shift),
            //0x50 => Some(ButtonType::Master),
            // Grid should add notes & add phrases
            //0x35 ..= 0x39 => Some(ButtonType::Grid(channel, 4 - (b[1]- 0x35))),
            //0x34 => Some(ButtonType::Indicator(channel)),
            //0x30 => Some(ButtonType::Red(channel)),
            0x31 => Some((self.surface_mut("pattern_zoom").unwrap(), Position::new(channel, 0))),
            0x32 => Some((self.surface_mut("pattern_length").unwrap(), Position::new(channel, 0))),
            // APC40 specific stuff
            //0x5B => Some(ButtonType::Play),
            //0x5C => Some(ButtonType::Stop),
            //0x3F => Some(ButtonType::Quantization),
            //0x5E => Some(ButtonType::Up),
            //0x5F => Some(ButtonType::Down),
            //0x60 => Some(ButtonType::Right),
            //0x61 => Some(ButtonType::Left),
            _ => None,
        }
    }

}

impl DeviceType for APC {
    fn port_name(&self) -> &'static str { self.apc_type.port_name() }

    fn surface(&self, name: &str) -> Option<&Surface> {
        self.surfaces.get(name)
    }

    fn surface_mut(&mut self, name: &str) -> Option<&mut Surface> {
        self.surfaces.get_mut(name)
    }

    fn process_midi_inputevent(&mut self, midi_input_event: MidiInputEvent, octothorpe: &mut Octothorpe) {
        let b = midi_input_event.bytes;

        // 0x06 = inquiry e, 0x02 = inquiry response 0x47 = akai manufacturer, 0x73 = APC40, 0x7b = APC20
        if b[0] == 0xF0 && b[3] == 0x06 && b[4] == 0x02 && b[5] == 0x47 && (b[6] == 0x73 || b[6] == 0x7b) {
            //println!("Inquiry response received, local_id: {:?}, device_id: {:?}", b[13], b[6]);
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

                // Get associated surface, and make it handle input
                if let Some((surface, position)) = self.recognized_button(channel, b[1]) {
                    let event = SurfaceEvent::new(midi_input_event.time, position, SurfaceEventType::Button(button_state));
                    surface.process_event(event);
                }
            },
            // Fader or pot adjusted
            0xB0 ..= 0xB8 => {
                /*
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
                };
                */
            },
            _ => (),
        };
    }

    // Get midi messages that should be output
    fn get_midi_messages(&mut self, cycle: &ProcessCycle) -> Vec<MidiMessage> {
        let mut messages = vec![];

        if self.introduced_at == 0 {
            // First run, not introduces
            if let (Some(local_id), Some(device_id)) = (self.local_id, self.device_id) {
                // ID is known, introduce ourselves
                self.introduced_at = cycle.time_start;
                //println!("Introducing {:?}", self.apc_type);
                messages.push(MidiMessage::new(0, vec![0xF0, 0x47, local_id, device_id, 0x60, 0x00, 0x04, 0x42, 0x00, 0x00, 0x00, 0xF7]))
            } else {
                // No ID know yet, inquire about device
                //println!("Inquiring {:?}", self.apc_type);
                messages.push(MidiMessage::new(0, vec![0xF0, 0x7E, 0x00, 0x06, 0x01, 0xF7]));
            }
        } else {
            // We have been introduced, wait a bit for APC to switch to alternate mode
        }

        messages
    }
}

#[derive(Debug)]
pub enum APCType {
    APC40,
    APC20,
}

impl APCType {
    fn port_name(&self) -> &'static str {
        match &self {
            Self::APC40 => "apc40",
            Self::APC20 => "apc20",
        }
    }
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
        device_type: impl DeviceType + Send + 'static,
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
    pub fn process_midi_input(&mut self, cycle: &ProcessCycle, octothorpe: &mut Octothorpe, devices: &Vec<Device>) {
        for message in self.input.iter(cycle.scope) {
            let midi_input_event = MidiInputEvent { time: cycle.frame_to_time(message.time), bytes: message.bytes };
            self.device_type.process_midi_inputevent(midi_input_event, octothorpe);
            //if let Some(input_event_type) = self.device_type.process_midi_inputevent(message) {
                //events.push(InputEvent::new(cycle.frame_to_time(message.time), input_event_type))
            //}
        }
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle, octothorpe: &mut Octothorpe, devices: &Vec<Device>) {
        let mut writer = self.output.writer(cycle.scope);
        let messages = self.device_type.get_midi_messages(cycle);

        for message in messages {
            writer.write(&message.to_jack_rawmidi());
        }
    }
}
