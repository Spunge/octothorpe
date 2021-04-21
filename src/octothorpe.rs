
use crate::*;

//pub struct ButtonEventMemory {
    //events: ,
//}

pub struct Octothorpe {
    pub interface: Interface,
    pub transport: Transport,
}

impl Octothorpe {
    pub fn new() -> Self {
        Self {
            interface: Interface::new(),
            transport: Transport::new(),
        }
    }
}
