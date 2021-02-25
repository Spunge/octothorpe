
//#![feature(drain_filter)]
#[macro_use]
extern crate matches;

extern crate jack;
use jack_sys as j;

pub mod controller;
pub mod hardware;
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
pub mod router;
pub mod tickrange;

// TODO - Save & load state on restart
//use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use sequencer::Sequencer;
use controller::*;
use hardware::*;
use surface::Surface;
use cycle::*;
use router::*;
use tickrange::*;

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

    //mixer: Mixer,
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

            //mixer: Mixer::new(client),
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
            println!("{:?}", _is_registered);
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

        self.apc20.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface);
        self.apc40.process_midi_input(&cycle, &mut self.sequencer, &mut self.surface);

        if cycle.is_rolling {
            self.sequencer.autoqueue_next_sequence(&cycle);
        }

        // Sequencer first at it will cache playing notes, these we can use for sequence visualization
        self.sequencer.output_midi(&cycle);
        //self.mixer.output_midi(&cycle);

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

