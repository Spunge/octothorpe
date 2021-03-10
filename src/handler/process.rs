
use crate::*;

pub struct ProcessHandler {
    surface: Arc<Mutex<Surface>>,
    transport: Arc<Mutex<Transport>>,
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
        surface: Arc<Mutex<Surface>>,
        transport: Arc<Mutex<Transport>>,
    ) -> Self {
        ProcessHandler {
            surface,
            transport,
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

        let mut surface = self.surface.lock().unwrap();
    
        let input_events = surface.input_events(scope);

        if input_events.len() > 0 {
            println!("{:?}", input_events);
        }

        /*
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

        */
        jack::Control::Continue
    }
}
