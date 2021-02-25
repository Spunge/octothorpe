
pub mod apc40;
pub mod apc20;

pub use self::apc40::APC40;
pub use self::apc20::APC20;

pub trait ControllerType {
    
}

pub struct Controller {
    pub system_source: jack::Port<jack::Unowned>,
    pub system_sink: Option<jack::Port<jack::Unowned>>,

    pub controller: Box<dyn ControllerType + Send>,
}

impl Controller {
    // We expect that system always reports capture port first, so we can create hardware
    // representations when we see the capture port and add the playback port later
    pub fn new(system_source: jack::Port<jack::Unowned>, controller: impl ControllerType + Send + 'static) -> Self {
        Controller {
            system_source,
            system_sink: None,

            controller: Box::new(controller),
        }
    }
}
