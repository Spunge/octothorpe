
//pub mod apc40;
//pub mod apc20;

//pub use self::apc40::OLD_APC40;
//pub use self::apc20::OLD_APC20;

use crate::*;

pub struct Grid { 
    pub width: u8,
    pub height: u8,
}

pub trait ControllerType {
    //fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface);
    //fn output_messages(&mut self, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) -> Vec<TimedMessage>;
    fn port_name(&self) -> &'static str;

    fn grid_mut(&mut self) -> Option<&mut Grid>;
    fn grid(&self) -> Option<&Grid>;

    fn process_rawmidi(&self, message: jack::RawMidi);
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

    fn grid_mut(&mut self) -> Option<&mut Grid> { Some(&mut self.grid) }
    fn grid(&self) -> Option<&Grid> { Some(&self.grid) }

    fn process_rawmidi(&self, message: jack::RawMidi) {
        // TODO - introduce here
        println!("{:?}", message);
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
    pub fn process_input(&mut self, cycle: &ProcessCycle, octothorpe: &mut Octothorpe, others: &Vec<Controller>) {
        //println!("{:?}", self.controller_type.port_name());
        let names: Vec<&str> = others.iter().map(|controller| controller.controller_type.port_name()).collect();
        //println!("others {:?}", names);

        self.input.iter(cycle.scope)
            .for_each(|message| {
                self.controller_type.process_rawmidi(message)
            })
    }

    pub fn output(&mut self, scope: &jack::ProcessScope) {
    
    }
}
