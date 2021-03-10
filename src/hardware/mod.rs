
//pub mod apc40;
//pub mod apc20;

//pub use self::apc40::OLD_APC40;
//pub use self::apc20::OLD_APC20;

use crate::inputevent::*;

pub struct Grid { 
    width: u8,
    height: u8,
}

pub trait ControllerType {
    //fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
    //fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
    fn port_name(&self) -> &'static str;

    fn grid(&mut self) -> Option<&mut Grid>;

    fn rawmidi_to_inputevent(&self, message: jack::RawMidi, other_controllers: &Vec<&mut Controller>) -> Option<InputEvent>;
}

pub struct APC {
    apc_type: Box<dyn APCType + Send>,
    is_introduced: bool,

    grid: Grid,
}

impl APC {
    pub fn new(apc_type: impl APCType + Send + 'static) -> Self {
        Self {
            apc_type: Box::new(apc_type),
            is_introduced: false,

            grid: Grid { width: 8, height: 5 },
        }
    }
}

impl ControllerType for APC {
    fn port_name(&self) -> &'static str { self.apc_type.port_name() }

    fn grid(&mut self) -> Option<&mut Grid> { Some(&mut self.grid) }

    fn rawmidi_to_inputevent(&self, message: jack::RawMidi, other_controllers: &Vec<&mut Controller>) -> Option<InputEvent> {
        println!("{:?}", message);
        println!("{:?}", other_controllers.len());
        None
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

pub struct Controller {
    pub system_source: jack::Port<jack::Unowned>,
    pub system_sink: Option<jack::Port<jack::Unowned>>,

    pub input: jack::Port<jack::MidiIn>,
    pub output: jack::Port<jack::MidiOut>,

    pub controller_type: Box<dyn ControllerType + Send>,
}

impl Controller {
    // We expect that system always reports capture port first, so we can create hardware
    // representations when we see the capture port and add the playback port later
    pub fn new(system_source: jack::Port<jack::Unowned>, client: &jack::Client, controller_type: impl ControllerType + Send + 'static) -> Self {
        let port_name = controller_type.port_name();

        let mut input_port_name = port_name.to_owned();
        input_port_name.push_str("_in");
        let mut output_port_name = port_name.to_owned();
        output_port_name.push_str("_out");

        Self {
            system_source,
            system_sink: None,

            input: client.register_port(input_port_name.as_str(), jack::MidiIn::default()).unwrap(),
            output: client.register_port(output_port_name.as_str(), jack::MidiOut::default()).unwrap(),

            controller_type: Box::new(controller_type),
        }
    }

    pub fn input_events(&mut self, scope: &jack::ProcessScope, other_controllers: &Vec<&mut Controller>) -> Vec<InputEvent> {
        self.input.iter(scope).filter_map(|message| self.controller_type.rawmidi_to_inputevent(message, other_controllers)).collect()
    }
}
