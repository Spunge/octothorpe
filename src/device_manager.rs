
use crate::*;

pub struct DeviceManager {
    octothorpe: Arc<Mutex<Octothorpe>>,
    port_registration_receiver: Receiver<(jack::PortId, bool)>,
    registered_ports: Vec<jack::Port<jack::Unowned>>,
}

impl DeviceManager {
    pub fn new(port_registration_receiver: Receiver<(jack::PortId, bool)>, octothorpe: Arc<Mutex<Octothorpe>>) -> Self {
        Self {
            octothorpe,
            port_registration_receiver,
            registered_ports: vec![],
        }
    }

    // TODO - We can't unwrap anything that's jack related without handling errors
    // FIXME - Sometimes jack reports no aliases for a port :|
    // Create device representations when devices connect
    pub fn register_port(&mut self, port: jack::Port<jack::Unowned>, client: &jack::Client) {
        // Create new device representation when input&output port both registered
        if port.flags().contains(jack::PortFlags::IS_OUTPUT) {
            let aliases = port.aliases().unwrap();

            // Remember system capture port registered
            // TODO - Use "recognized" device config for this, meaning we could pass a config
            // file that also contains offsets for example
            if let Some(alias) = aliases.iter().find(|alias| alias.contains("APC40") || alias.contains("APC20")) {
                self.registered_ports.push(port);
            }
        } else {
            // Sink port registered, add it to correct device
            let capture_port_name = port.name().unwrap().replace("playback", "capture");

            // Got input port aswell?
            let capture_port = self.registered_ports.iter_mut().enumerate().find(|(_index, port)| port.name().unwrap() == capture_port_name);
            if let Some((index, _port)) = capture_port {
                let capture_port = self.registered_ports.swap_remove(index);

                // Add found device
                let is_apc40 = port.aliases().unwrap().iter().find(|alias| alias.contains("APC40")).is_some();
                let device_type = if is_apc40 { APC::new(APC40::new()) } else { APC::new(APC20::new()) };

                //println!("adding device {:?}", capture_port.aliases().unwrap().first().unwrap());
                self.octothorpe.lock().unwrap().add_device(Device::new(client, capture_port, port, device_type));
            }
        }
    }

    // Destroy device when ports disconnect
    pub fn deregister_port(&mut self, port: jack::Port<jack::Unowned>, client: &jack::Client) {
        // Get lock
        let mut octothorpe = self.octothorpe.lock().unwrap();

        // Only destroy on output port disconnect as devices have multiple ports
        let device = octothorpe.devices.iter().enumerate()
            .find(|(_index, device)| device.system_source.name().unwrap() == port.name().unwrap());

        // Deregister jack ports on removing device
        if let Some((index, device)) = device {
            //println!("removing device {:?}", &device.system_source.name().unwrap());
            // We get a new port representation as we need to own port that we pass to client,
            // and we can't own octo as it sits behind mutex
            let input_port = client.port_by_name(&device.input.name().unwrap()).unwrap();
            client.unregister_port(input_port);
            let output_port = client.port_by_name(&device.output.name().unwrap()).unwrap();
            client.unregister_port(output_port);

            // Remove device from octo
            octothorpe.remove_device(index);
        }
    }

    pub fn watch_registrations(&mut self, client: &jack::Client) {
        // Currently existing registered ports should also be handled
        let ports: Vec<jack::Port<jack::Unowned>> = client
            .ports(None, Some("midi"), jack::PortFlags::IS_PHYSICAL)
            .into_iter()
            .map(|name| client.port_by_name(&name).unwrap())
            .collect();

        for port in ports {
            self.register_port(port, client);
        }

        // Wait for notifications about new ports
        while let Ok((port_id, is_registered)) = self.port_registration_receiver.recv() {
            let port = client.port_by_id(port_id).unwrap();

            // We're not interested in ports we are creating ourselves
            if client.is_mine(&port) {
                continue
            }

            if is_registered {
                // We're only interested in physical midi ports
                if port.flags().contains(jack::PortFlags::IS_PHYSICAL) && port.port_type().unwrap().contains("midi") {
                    println!("{:?} registered", port.name().unwrap());
                    self.register_port(port, client);
                }
            } else {
                // On deregister, no port info is known, so we'll have to check every port
                println!("{:?} deregistered", port.name().unwrap());
                self.deregister_port(port, client);
            }
        }
    }
}

