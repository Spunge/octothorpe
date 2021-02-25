
use std::sync::mpsc::{Sender, Receiver};

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
    pub fn new(connection_receive: Receiver<(jack::Port<jack::Unowned>, bool)>, introduction_send: Sender<(jack::Port<jack::Unowned>, bool)>) -> Self {
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

        println!("{:?}", port.aliases().unwrap());

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
        //for port_name in client.ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL).iter() {
            //if let Some(port) = client.port_by_name(port_name) {
                //self.handle_port_registration(client, port, true);
            //}
        //}

        // Wait for notifications about new ports
        while let Ok((port, is_registered)) = self.connection_receive.recv() {
            //if port.port_type().unwrap().contains("midi") && port.flags().contains(jack::PortFlags::IS_PHYSICAL) {
                //self.handle_port_registration(client, port, is_registered);
            //}
        }
    }
}

