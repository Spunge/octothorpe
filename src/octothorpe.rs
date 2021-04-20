
use crate::*;

//pub struct ButtonEventMemory {
    //events: ,
//}

pub struct Octothorpe {
    devices: Arc<Mutex<Vec<Device>>>,

    pub interface: Interface,
    pub transport: Transport,
}

impl Octothorpe {
    pub fn new(devices: Arc<Mutex<Vec<Device>>>) -> Self {
        Self {
            devices,

            interface: Interface::new(),
            transport: Transport::new(),
        }
    }

    pub fn process_midi_input(&mut self, cycle: &ProcessCycle) {
        for (index, device) in self.devices.lock().unwrap().iter_mut().enumerate() {
            let events = device.process_midi_input(cycle);
        }
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        for device in self.devices.lock().unwrap().iter_mut() {
            device.output_midi(cycle);
        }
    }
}
