
use crate::*;

//pub struct ButtonEventMemory {
    //events: ,
//}

pub struct Octothorpe {
    pub devices: Vec<Device>,

    pub interface: Interface,
    pub transport: Transport,
}

impl Octothorpe {
    pub fn new() -> Self {
        Self {
            devices: vec![],

            interface: Interface::new(),
            transport: Transport::new(),
        }
    }

    pub fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }

    pub fn remove_device(&mut self, index: usize) {
        self.devices.remove(index);
    }

    pub fn process_midi_input(&mut self, cycle: &ProcessCycle) {
        for (index, device) in self.devices.iter_mut().enumerate() {
            let events = device.process_midi_input(cycle);
            if events.len() > 0 {
                println!("{:?}", events);
            }
        }
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle) {
        for device in self.devices.iter_mut() {
            device.output_midi(cycle);
        }
    }
}
