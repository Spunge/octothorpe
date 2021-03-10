
use crate::*;

/*
 * NotificationHandler is called back on certain jack events
 */
pub struct NotificationHandler {
    port_registration_sender: Sender<(jack::Port<jack::Unowned>, bool)>,
}

// We use a channel to notify main thread of newly connected system ports
// I tried to create new controller representations and their corresponding ports from the
// notification thread first, but jack errors, telling me that i can't query the jack server from
// the notification thread (to create ports)
impl NotificationHandler {
    pub fn new(client: &jack::Client, port_registration_sender: Sender<(jack::Port<jack::Unowned>, bool)>) -> Self {
        let mut handler = NotificationHandler { 
            port_registration_sender 
        };

        // Currently existing registered ports should also be handled
        let ports: Vec<jack::Port<jack::Unowned>> = client
            .ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL)
            .into_iter()
            .map(|name| client.port_by_name(&name).unwrap())
            .collect();

        for port in ports {
            handler.port_registration_sender.send((port, true));
        }

        handler
    }
}

// Jack Notification handler port registration callback
impl jack::NotificationHandler for NotificationHandler {
    fn port_registration(&mut self, client: &jack::Client, port_id: jack::PortId, is_registered: bool) {
        let port = client.port_by_id(port_id).unwrap();
        self.port_registration_sender.send((port, is_registered));
    }
}

