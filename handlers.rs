
pub struct ProcessHandler<'a> {
    pub controller: &'a super::controller::Controller,
    pub midi_in: &'a jack::Port<jack::MidiIn>, 
    pub midi_out: &'a mut jack::Port<jack::MidiOut>,
}

impl<'a> jack::ProcessHandler for ProcessHandler<'a> {
    fn process(&mut self, _client: &jack::Client, process_scope: &jack::ProcessScope) -> jack::Control {
        // Handle input & output in controller
        self.controller.process_midi_input(self.midi_in.iter(process_scope));
        self.controller.write_midi_output(self.midi_out.writer(process_scope));

        jack::Control::Continue
    }
}

pub struct NotificationHandler<'a> {
    pub controller: &'a super::controller::Controller,
}

impl<'a> jack::NotificationHandler for NotificationHandler<'a> {
    fn ports_connected(&mut self, client: &jack::Client, port_id_a: jack::PortId, port_id_b: jack::PortId, are_connected: bool) {
        // Get ports from client
        let port_a = client.port_by_id(port_id_a).unwrap();
        let port_b = client.port_by_id(port_id_b).unwrap();

        // Only interested in our own ports
        if (client.is_mine(&port_a) || client.is_mine(&port_b)) && are_connected {
            // Make controller say hello
            self.controller.identify();
        }
    }
}

