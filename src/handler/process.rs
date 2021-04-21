
use crate::*;

pub struct ProcessHandler {
    octothorpe: Arc<Mutex<Octothorpe>>,
    devices: Arc<Mutex<Vec<Device>>>,
    // Controllers
    //apc20: APC20,
    //apc40: APC40,

    //mixer: Mixer,
    //sequencer: Sequencer,
    //surface: Surface,
}


impl ProcessHandler {
    pub fn new(
        //_timebase_sender: Sender<f64>,
        //client: &jack::Client
        octothorpe: Arc<Mutex<Octothorpe>>,
        devices: Arc<Mutex<Vec<Device>>>,
    ) -> Self {
        ProcessHandler {
            octothorpe,
            devices,
            //apc20: APC20::new(client),
            //apc40: APC40::new(client),

            //mixer: Mixer::new(client),
            //sequencer: Sequencer::new(client),
            //surface: Surface::new(),
        }
    }
}

impl jack::ProcessHandler for ProcessHandler {
    fn process(&mut self, client: &jack::Client, scope: &jack::ProcessScope) -> jack::Control {

        let cycle = ProcessCycle::new(client, scope);

        let mut devices = self.devices.lock().unwrap();
        let mut octothorpe = self.octothorpe.lock().unwrap();

        // Grab devices from index 0 and push them onto the end, this way we can pass all other
        // devices to the device, so it can change it's behaviour based on other controllers present
        for _index in 0..devices.len() {
            let mut device = devices.remove(0);
            device.process_midi_input(&cycle, &mut octothorpe, &devices);
            devices.push(device);
        }
        for _index in 0..devices.len() {
            let mut device = devices.remove(0);
            device.output_midi(&cycle, &mut octothorpe, &devices);
            devices.push(device);
        }

        // TODO - Sequencer output

        /*
        // Get something representing this process cycle
        let cycle = ProcessCycle::new(client, scope);

        while let Ok((port, _is_registered)) = self.introduction_receiver.try_recv() {
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

        */
        jack::Control::Continue
    }
}
