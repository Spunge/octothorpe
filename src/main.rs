
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;
use jack_sys as j;

pub mod controller;
pub mod message;
pub mod sequencer;
pub mod cycle;
pub mod channel;
pub mod loopable;
pub mod sequence;
pub mod surface;
pub mod port;
pub mod mixer;
pub mod events;
pub mod instrument;

use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use sequencer::Sequencer;
use controller::*;
use mixer::*;
use surface::Surface;
use cycle::*;

#[derive(Copy, Clone, Debug)]
pub struct TickRange {
    pub start: u32,
    pub stop: u32,
}

impl TickRange {
    fn new(start: u32, stop: u32) -> Self {
        Self { start, stop }
    }

    pub fn plus(&self, delta: u32) -> Self {
        Self {
            start: self.start + delta,
            stop: self.stop + delta,
        }
    }

    pub fn minus(&self, delta: u32) -> Self {
        Self {
            start: self.start - delta,
            stop: self.stop - delta,
        }
    }

    pub fn contains(&self, tick: u32) -> bool {
        tick >= self.start && tick < self.stop
    }

    pub fn overlaps(&self, other: &TickRange) -> bool {
        self.start < other.stop && self.stop > other.start
    }

    pub fn length(&self) -> u32 {
        self.stop - self.start
    }
}

pub struct TimebaseHandler {
    beats_per_minute: f64,
    beats_per_bar: f32,
    beat_type: f32,
    is_up_to_date: bool,

    //receiver: Receiver<f64>,
}

impl TimebaseHandler {
    pub const TICKS_PER_BEAT: f64 = 1920.0;

    pub fn new(_: Receiver<f64>) -> Self {
        TimebaseHandler {
            beats_per_minute: 138.0,
            is_up_to_date: false,
            beats_per_bar: 4.0,
            beat_type: 4.0,
            //receiver,
        }
    }
}

impl jack::TimebaseHandler for TimebaseHandler {
    fn timebase(&mut self, _: &jack::Client, _state: jack::TransportState, _n_frames: jack::Frames, pos: *mut jack::Position, is_new_pos: bool) {
        unsafe {
            // Set position type
            (*pos).valid = j::JackPositionBBT;

            // BPM changed?
            //if ! is_new_pos && (*pos).beats_per_minute != self.beats_per_minute {
                //println!("{:?}", (*pos).beats_per_minute);
            //}

            // Only update timebase when we are asked for it, or when our state changed
            if is_new_pos || ! self.is_up_to_date {
                (*pos).beats_per_bar = self.beats_per_bar;
                (*pos).ticks_per_beat = Self::TICKS_PER_BEAT;
                (*pos).beat_type = self.beat_type;
                (*pos).beats_per_minute = self.beats_per_minute;

                self.is_up_to_date = true;
            }

            let abs_tick = ProcessCycle::frame_to_tick(*pos, (*pos).frame);
            let abs_beat = abs_tick / (*pos).ticks_per_beat;

            // Plus 1 as humans tend not to count from 0
            (*pos).bar = (abs_beat / (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).beat = (abs_beat % (*pos).beats_per_bar as f64) as i32 + 1;
            (*pos).bar_start_tick = (abs_beat as i32 * (*pos).ticks_per_beat as i32) as f64;
            (*pos).tick = abs_tick as i32 - (*pos).bar_start_tick as i32;
        }
    }
}


pub struct ProcessHandler {
    // Controllers
    apc20: APC20,
    apc40: APC40,

    mixer: Mixer,
    sequencer: Sequencer,
    surface: Surface,

    introduction_receiver: Receiver<(jack::PortId, bool)>,
}

impl ProcessHandler {
    pub fn new(
        introduction_receiver: Receiver<(jack::PortId, bool)>,
        _timebase_sender: Sender<f64>,
        client: &jack::Client
    ) -> Self {
        ProcessHandler { 
            apc20: APC20::new(client),
            apc40: APC40::new(client),

            mixer: Mixer::new(client),
            sequencer: Sequencer::new(client), 
            surface: Surface::new(),
            introduction_receiver,
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, client: &jack::Client, scope: &jack::ProcessScope) -> jack::Control {
        // Get something representing this process cycle
        let cycle = ProcessCycle::new(client, scope);

        while let result = self.introduction_receiver.try_recv() {
            match result {
                Result::Ok((port_id, is_registered)) => {
                    let port = client.port_by_id(port_id);
                    println!("INTRO {:?}", port);
                },
                Result::Err(_) => break,
            }
        }

        self.apc20.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface, &mut self.mixer);
        self.apc40.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface, &mut self.mixer);

        if cycle.is_rolling {
            self.sequencer.autoqueue_next_sequence(&cycle);
        }

        // Sequencer first at it will cache playing notes, these we can use for sequence visualization
        self.sequencer.output_midi(&cycle);
        self.mixer.output_midi(&cycle);

        self.apc20.output_midi(&cycle, &mut self.sequencer, &mut self.surface);
        self.apc40.output_midi(&cycle, &mut self.sequencer, &mut self.surface);

        jack::Control::Continue
    }
}

// Get JACK midi port representations
fn get_midi_ports(client: &jack::Client, port_flags: jack::PortFlags) -> Vec<jack::Port<jack::Unowned>> {
    return client
        .ports(None, Some("midi"), port_flags)
        .iter()
        // Strip own ports
        .filter(|port_name| ! port_name.contains("octothorpe"))
        // Get jack portSpecs
        .map(|port_name| client.port_by_name(&port_name).unwrap())
        .collect();
}

fn find_port_with_alias<'a>(ports: &'a Vec<jack::Port<jack::Unowned>>, alias_pattern: &str) -> Option<&'a jack::Port<jack::Unowned>> {
    ports.iter().find(|port| {
        port.aliases().unwrap().iter().find(|alias| alias.contains(alias_pattern)).is_some()
    })
}

pub struct NotificationHandler {
    sender: Sender<(jack::PortId, bool)>,
}

impl NotificationHandler {
    pub fn new(
        sender: Sender<(jack::PortId, bool)>,
        //client: &jack::Client
    ) -> Self {
        NotificationHandler { 
            sender,
        }
    }
}

impl jack::NotificationHandler for NotificationHandler {
    fn port_registration(&mut self, client: &jack::Client, port_id: jack::PortId, is_registered: bool) {
        self.sender.send((port_id, is_registered));
        //let port = client.port_by_id(port_id);
        //println!("{:?}", port);
        //println!("{:?}", is_registered);
    }
}


// Connect octothorpe to external midi devices
fn connect_midi_ports(client: &jack::Client) {
    // For me it seems logical to call ports that read midi from outside "input", 
    // But jack has other ideas, it calls ports that output midi "output", which is why i switch them here
    let input_ports = get_midi_ports(client, jack::PortFlags::IS_OUTPUT);
    let output_ports = get_midi_ports(client, jack::PortFlags::IS_INPUT);
    
    // TODO - brevity?
    if let Some(port) = find_port_with_alias(&input_ports, "APC20") {
        println!("connect apc20");
        client.connect_ports_by_name(&port.name().unwrap(), "octothorpe:apc20_in");
    }
    if let Some(port) = find_port_with_alias(&output_ports, "APC20") {
        println!("connect apc20 outpu");
        client.connect_ports_by_name("octothorpe:apc20_out", &port.name().unwrap());
    }
    if let Some(port) = find_port_with_alias(&input_ports, "APC40") {
        println!("connect apc40");
        client.connect_ports_by_name(&port.name().unwrap(), "octothorpe:apc40_in");
    }
    if let Some(port) = find_port_with_alias(&output_ports, "APC40") {
        println!("connect apc40 output");
        client.connect_ports_by_name("octothorpe:apc40_out", &port.name().unwrap());
    }

    // Get all input ports that are not APC ports
    let external_input_ports: Vec<&jack::Port<jack::Unowned>> = input_ports
        .iter()
        .filter(|port| port.aliases().unwrap().iter().find(|alias| alias.contains("APC")).is_none())
        .collect();
    let external_output_ports: Vec<&jack::Port<jack::Unowned>> = output_ports
        .iter()
        .filter(|port| port.aliases().unwrap().iter().find(|alias| alias.contains("APC")).is_none())
        .collect();

    // Connect each input to every output, except the output that has the same number as the input
    // port. This way we can hook up devices with input & output without sending them the midi
    // events they output themselves
    for input_port in external_input_ports {
        let input_port_name = input_port.name().unwrap();
        let input_port_num = input_port_name.split("_").last().unwrap();

        for output_port in &external_output_ports {
            let output_port_name = output_port.name().unwrap();
            let output_port_num = output_port_name.split("_").last().unwrap();

            if(input_port_num != output_port_num) {
                client.connect_ports_by_name(&input_port_name, &output_port_name);
            }
        }
    }

    // Hook up every octothorpe channel output to every system output port. We have seperated the
    // channel outputs so we can have control over what channel gets routed where
    for num in 0..16 {
        for output_port in &external_output_ports {
            let output_port_name = output_port.name().unwrap();

            client.connect_ports_by_name(&format!("octothorpe:channel_{}", num), &output_port_name);
        }
    }
}

/*
 * Router will process port connected signals from the notification handler. It will connect the
 * octothorpe ports to other clients and vice versa, also, it will notify the processhandler of (re-)connected APC's.
 *
 * I was unable to make jack connections from the process handler, therefore this is done here, and
 * executed from the main thread
 */
pub struct Router {
    connection_receive: Receiver<(jack::PortId, bool)>,
    introduction_send: Sender<(jack::PortId, bool)>,
}

impl Router {
    fn new(connection_receive: Receiver<(jack::PortId, bool)>, introduction_send: Sender<(jack::PortId, bool)>) -> Self {
        Router { 
            connection_receive,
            introduction_send,
        }
    }

    // Connect a port to it's intended input / output
    pub fn connect(&self, client: &jack::Client, port: jack::Port<jack::Unowned>) {
        // IS_OUTPUT = midi_capture
        // IS_INPUT = midi_playback
        let known_ports = vec![
            // Part of alias, port flags, connect to port
            ("APC40", jack::PortFlags::IS_OUTPUT, "octothorpe:apc40_in"),
            ("APC40", jack::PortFlags::IS_INPUT, "octothorpe:apc40_out"),
            ("APC20", jack::PortFlags::IS_OUTPUT, "octothorpe:apc40_in"),
            ("APC20", jack::PortFlags::IS_INPUT, "octothorpe:apc40_out"),
        ];

        for (alias_pattern, flag, target_port_name) in known_ports {
            let has_alias_with_pattern = port.aliases().unwrap().iter().find(|alias| alias.contains("APC40")).is_some();
            if has_alias_with_pattern && port.flags().contains(flag) {
                if flag == jack::PortFlags::IS_INPUT {
                    client.connect_ports_by_name(target_port_name, &port.name().unwrap());
                }
                if flag == jack::PortFlags::IS_OUTPUT {
                    client.connect_ports_by_name(&port.name().unwrap(), target_port_name);
                }
                // TODO - Send introduction
            }
        }

        // TODO - Connect all "unknown" ports (midi interfaces etc)

        /*
        if port.flags().contains(jack::PortFlags::IS_PHYSICAL) {
            if port.flags().contains(jack::PortFlags::IS_OUTPUT) {
                if {
                    println!("APC40 INPUT {:?} {:?}", port.name().unwrap(), port.flags());
                } else if port.aliases().unwrap().iter().find(|alias| alias.contains("APC20")).is_some() {
                    println!("APC20 INPUT {:?} {:?}", port.name().unwrap(), port.flags());
                } else {
                    println!("UNKNOWN INPUT {:?} {:?}", port.name().unwrap(), port.flags());
                }
            }
            if port.flags().contains(jack::PortFlags::IS_INPUT) {
                if port.aliases().unwrap().iter().find(|alias| alias.contains("APC40")).is_some() {
                    println!("APC40 OUTPUT {:?} {:?}", port.name().unwrap(), port.flags());
                    //self.introduction_send((port.port_id()))
                } else if port.aliases().unwrap().iter().find(|alias| alias.contains("APC20")).is_some() {
                    println!("APC20 OUTPUT {:?} {:?}", port.name().unwrap(), port.flags());
                } else {
                    println!("UNKNOWN OUTPUT {:?} {:?}", port.name().unwrap(), port.flags());
                }
            }


            //client.connect_ports_by_name(port.name().unwrap(), "octothorpe:apc40_in")
        }
        */
    }

    // Start routing, this function halts and waits for notifications of connected midi ports
    pub fn start(&mut self, client: &jack::Client) {
        // Connect existing ports
        for port_name in client.ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL).iter() {
            if let Some(port) = client.port_by_name(port_name) {
                self.connect(client, port);
            }
        }

        // Wait for notifications about new ports
        while let result = self.connection_receive.recv() {
            // New port registered
            match result {
                Result::Ok((port_id, is_registered)) => {
                    if let Some(port) = client.port_by_id(port_id) {
                        self.connect(client, port);
                    }
                },
                Result::Err(_) => (),
            }
        }
    
    }
}


fn main() {
    // Setup client
    let (client, _status) =
        jack::Client::new("octothorpe", jack::ClientOptions::NO_START_SERVER).unwrap();

    let (timebase_sender, timebase_receiver) = channel();
    let (introduction_send, introduction_receive) = channel();
    let (connection_send, connection_receive) = channel();

    let mut router = Router::new(connection_receive, introduction_send);

    let notificationhandler = NotificationHandler::new(connection_send);
    let timebasehandler = TimebaseHandler::new(timebase_receiver);
    let processhandler = ProcessHandler::new(introduction_receive, timebase_sender, &client);

    // Activate client
    let async_client = client
        .activate_async(notificationhandler, processhandler, timebasehandler)
        .unwrap();

    // Connect Octo to APC's and connect system ports
    connect_midi_ports(async_client.as_client());


    //client.connect_ports_by_name("system:midi_capture_16", "system:midi_playback_16");
    //while let test = introduction_receiver.recv() {
        //connect_midi_ports(async_client.as_client());
        //println!("{:?}", test);
    //}

    // Wait for user to input string
    //loop {
        //let mut user_input = String::new();
        //io::stdin().read_line(&mut user_input).ok();
    //}
    router.start(async_client.as_client());
}

