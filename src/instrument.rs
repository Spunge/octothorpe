
use super::pattern::Pattern;
use super::phrase::Phrase;

#[derive(Debug)]
pub struct RecordedMessage {
    time: u32,
    channel: u8,
    key: u8,
    velocity: u8,
}

pub struct Instrument {
    // TODO - these are public as we're testing with premade patterns
    pub patterns: [Pattern; 5],
    pub phrases: [Phrase; 5],

    pub phrase: usize,
    pub pattern: usize,

    pub knob_group: u8,
    knob_values: [u8; 64],

    pub recorded_messages: Vec<RecordedMessage>,
}

impl Instrument {
    pub fn new(c: u8) -> Self {
        let patterns = [ Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), Pattern::new(c), ];
        let phrases = [ Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), Phrase::new(), ];

        Instrument {
            phrases,
            patterns,
            phrase: 0,
            pattern: 0,

            // There's 4 knob groups, this way we can have knobs * 4 !
            knob_group: 0,
            knob_values: [0; 64],

            recorded_messages: vec![],
        }
    }

    pub fn pattern(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern]
    }

    pub fn phrase(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase]
    }

    pub fn switch_knob_group(&mut self, group: u8) {
        self.knob_group = group;
    }

    pub fn set_knob_value(&mut self, index: u8, value: u8) -> u8 {
        let knob = self.knob_group * 16 + index;
        self.knob_values[knob as usize] = value;
        knob
    }

    pub fn get_knob_values(&self) -> &[u8] {
        let start = self.knob_group as usize * 16;
        let end = start as usize + 16;
        &self.knob_values[start .. end]
    }

    pub fn knob_value_changed(&mut self, knob: u8, value: u8) -> Option<u8> {
        if self.knob_values[knob as usize] != value {
            self.knob_values[knob as usize] = value;
            Some(value)
        } else {
            None
        }
    }

    pub fn record_message(&mut self, time: u32, channel: u8, key: u8, velocity: u8) {
        //println!("0x{:X}, 0x{:X}, 0x{:X}", message.bytes[0], message.bytes[1], message.bytes[2]);

        let recorded_message = RecordedMessage { time, channel, key, velocity, };

        // TODO - if note is note down, merge it with previous note on on the same key
        if channel == 0x80 {
            let index = self.recorded_messages.iter().position(|message| {
                message.key == recorded_message.key && message.channel == 0x90
            }).unwrap();

            let message = &self.recorded_messages[index];

            self.patterns.iter_mut()
                .filter(|pattern| pattern.is_recording)
                .for_each(move |pattern| {
                    pattern.toggle_note(
                        message.time,
                        recorded_message.time,
                        recorded_message.key,
                        message.velocity,
                        recorded_message.velocity,
                    );
                });

            println!("Started at {:?}", self.recorded_messages[index].time);

            self.recorded_messages.remove(index);
        } else {
            self.recorded_messages.push(recorded_message);
        }
        //println!("{:?}", self.recorded_messages);

        //self.patterns.iter_mut()
            //.filter(|pattern| pattern.is_recording)
            //.for_each(|pattern| pattern.record_message(cycle_start, message));
    }
}
