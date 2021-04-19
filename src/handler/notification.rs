
use crate::*;

/*
 * NotificationHandler is called back on certain jack events
 */
pub struct NotificationHandler {
    port_registration_sender: Sender<(jack::PortId, bool)>,
}

// We use a channel to notify main thread of newly connected system ports
// I tried to create new controller representations and their corresponding ports from the
// notification thread first, but jack errors, telling me that i can't query the jack server from
// the notification thread (to create ports)
impl NotificationHandler {
    pub fn new(port_registration_sender: Sender<(jack::PortId, bool)>) -> Self {
        Self { port_registration_sender }
    }
}

// Jack Notification handler port registration callback
impl jack::NotificationHandler for NotificationHandler {
    fn port_registration(&mut self, client: &jack::Client, port_id: jack::PortId, is_registered: bool) {
        self.port_registration_sender.send((port_id, is_registered));
    }
}

