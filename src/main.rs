
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

// TODO - Save & load state on restart
//use std::io;
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

    introduction_receiver: Receiver<(jack::Port<jack::Unowned>, bool)>,
}

impl ProcessHandler {
    pub fn new(
        introduction_receiver: Receiver<(jack::Port<jack::Unowned>, bool)>,
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

        while let Ok((port, _is_registered)) = self.introduction_receiver.try_recv() {
            // TODO - Use is_registered to create & destroy controller structs
            // @important - for now we only get is_registered = true, as for now, we only
            // connect new ports
            let is_apc40 = port.aliases().unwrap().iter()
                .find(|alias| alias.contains("APC40"))
                .is_some();

            // For now we know for sure that we have 2 controllers
            if is_apc40 {
                self.apc40.set_identified_cycles(0);
            } else {
                self.apc20.set_identified_cycles(0);
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

/*
 * NotificationHandler is called back on certain jack events
 */
pub struct NotificationHandler {
    sender: Sender<(jack::Port<jack::Unowned>, bool)>,
}

impl NotificationHandler {
    pub fn new(
        sender: Sender<(jack::Port<jack::Unowned>, bool)>,
        //client: &jack::Client
    ) -> Self {
        NotificationHandler {
            sender,
        }
    }
}

// We're only interested in letting the main thread know about new jack ports
impl jack::NotificationHandler for NotificationHandler {
    fn port_registration(&mut self, client: &jack::Client, port_id: jack::PortId, is_registered: bool) {
        let port = client.port_by_id(port_id).unwrap();
        self.sender.send((port, is_registered)).unwrap();
    }
}

/*
 * Router will process port connected signals from the notification handler. It will connect the
 * octothorpe ports to other clients and vice versa, also, it will notify the processhandler of (re-)connected APC's.
 *
 * I was unable to make jack connections from the process handler, therefore this is done here, and
 * executed from the main thread
 */
pub struct Router<'a> {
    connection_receive: Receiver<(jack::Port<jack::Unowned>, bool)>,
    introduction_send: Sender<(jack::Port<jack::Unowned>, bool)>,
    port_designations: Vec<(&'a str, jack::PortFlags, &'a str)>,
}

impl Router<'_> {
    fn new(connection_receive: Receiver<(jack::Port<jack::Unowned>, bool)>, introduction_send: Sender<(jack::Port<jack::Unowned>, bool)>) -> Self {
        Router {
            connection_receive,
            introduction_send,
            port_designations: vec![
                // Part of alias, port flags, connect to port
                ("APC40", jack::PortFlags::IS_OUTPUT, "octothorpe:apc40_in"),
                ("APC40", jack::PortFlags::IS_INPUT, "octothorpe:apc40_out"),
                ("APC20", jack::PortFlags::IS_OUTPUT, "octothorpe:apc20_in"),
                ("APC20", jack::PortFlags::IS_INPUT, "octothorpe:apc20_out"),
            ],
        }
    }

    // Does this jack port match with controller target port?
    pub fn matches_port_designation(port: &jack::Port<jack::Unowned>, port_designation: &(&str, jack::PortFlags, &str)) -> bool {
        let (alias_pattern, flag, _) = port_designation;

        let has_alias_with_pattern = port.aliases().unwrap().iter()
            .find(|alias| alias.contains(alias_pattern))
            .is_some();

        let has_flag = port.flags().contains(*flag);

        has_alias_with_pattern && has_flag
    }

    // Is jack port a controller port that we know?
    pub fn controller_target_port(&self, port: &jack::Port<jack::Unowned>) -> Option<String> {
        self.port_designations.iter()
            .find(|port_designation| Self::matches_port_designation(port, port_designation))
            .and_then(|(_, _, target_port)| Some(String::from(*target_port)))
    }

    // TODO - Non-controller ports should be connected to all non-controller ports except
    // the input/output port with the same number.
    // This way we can use multiple midi instruments as 1 instrument
    pub fn default_target_ports(&self, client: &jack::Client, port: &jack::Port<jack::Unowned>) -> Vec<String> {
        // Get all physical ports that this port should be connected to
        let mut ports: Vec<String> = client
            .ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL)
            .into_iter()
            .filter(|port_name| {
                let target_port = client.port_by_name(&port_name).unwrap();

                let should_contain_flag = if port.flags().contains(jack::PortFlags::IS_OUTPUT) { 
                    jack::PortFlags::IS_INPUT 
                } else { 
                    jack::PortFlags::IS_OUTPUT 
                };

                // We only want to connect output to input & vice versa
                let is_opposite_port = target_port.flags().contains(should_contain_flag);
                // We don't want to patch input to output of same device
                let is_same_port_number = target_port.name().unwrap().split("_").last().unwrap() == port.name().unwrap().split("_").last().unwrap();
                    // We're only interested in non-controller ports
                let is_designated_port = self.port_designations.iter()
                    .find(|port_designation| Self::matches_port_designation(&target_port, port_designation))
                    .is_some();

                is_opposite_port && ! is_same_port_number && ! is_designated_port
            })
            .collect();

        // Is this port a midi output port? If so, connect it to our sequencer channels
        if port.flags().contains(jack::PortFlags::IS_INPUT) {
            for num in 0..16 {
                ports.push(format!("octothorpe:channel_{}", num))
            }
        }

        ports
    }

    // Connect a port to it's intended input / output
    pub fn handle_port_registration(&self, client: &jack::Client, port: jack::Port<jack::Unowned>, is_registered: bool) {
        // TODO - We would like processhandler to create & destroy controller structs on connecting
        // /distconnecting controllers, for now though, we know that we have 1 APC20 & 1 APC40
        if ! is_registered {
            return
        }

        let mut should_reintroduce = false;

        // What ports to connect to? Also, when connecting controller ports, let ProcessHandler
        // know that it should re-introduce with controllers
        let target_ports = if let Some(target_port) = self.controller_target_port(&port) {
            should_reintroduce = true;

            vec![target_port]
        } else {
            self.default_target_ports(client, &port)
        };

        // Make actual connections
        for target_port_name in target_ports.iter() {
            // connect_ports_by_name will fail if you don't pass capture first and playback second
            if port.flags().contains(jack::PortFlags::IS_OUTPUT) {
                client.connect_ports_by_name(&port.name().unwrap(), target_port_name).unwrap();
            } else {
                client.connect_ports_by_name(target_port_name, &port.name().unwrap()).unwrap();
            }
        }

        if should_reintroduce {
            self.introduction_send.send((port, is_registered)).unwrap();
        }
    }

    // Start routing, this function halts and waits for notifications of connected midi ports
    pub fn start(&mut self, client: &jack::Client) {
        // Connect existing ports
        for port_name in client.ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL).iter() {
            if let Some(port) = client.port_by_name(port_name) {
                self.handle_port_registration(client, port, true);
            }
        }

        // Wait for notifications about new ports
        while let Ok((port, is_registered)) = self.connection_receive.recv() {
            if port.port_type().unwrap().contains("midi") && port.flags().contains(jack::PortFlags::IS_PHYSICAL) {
                self.handle_port_registration(client, port, is_registered);
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

    // Start router that will listen for new ports & handle connections
    router.start(async_client.as_client());
}

