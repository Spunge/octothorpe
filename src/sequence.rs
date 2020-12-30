
use super::channel::Channel;
use super::loopable::*;

pub struct Sequence {
    // Phrase that's playing for channel, array index = channel
    phrases: [Option<u8>; 16],
    active: [bool; 16],
}

impl Sequence {
    pub fn new(phrase: u8) -> Self {
        Sequence {
            phrases: [Some(phrase); 16],
            active: [true; 16],
        }
    }

    pub fn phrases(&self) -> &[Option<u8>; 16] {
        &self.phrases
    }

    pub fn get_phrase(&self, channel: usize) -> Option<u8> {
        self.phrases[channel]
    }

    pub fn set_phrases(&mut self, phrase: u8) {
        self.phrases = [Some(phrase); 16];
    }

    pub fn set_phrase(&mut self, channel: usize, phrase: u8) {
        self.phrases[channel] = Some(phrase);
    }

    pub fn unset_phrase(&mut self, channel: usize) {
        self.phrases[channel] = None;
    }

    pub fn active_phrase(&self, channel: usize) -> Option<u8> {
        self.phrases[channel].and_then(|phrase| if self.active[channel] { Some(phrase) } else { None })
    }

    pub fn toggle_active(&mut self, channel: usize) {
        self.active[channel as usize] = ! self.active[channel as usize];
    }

    pub fn length(&self, channels: &[Channel]) -> u32 {
        self.phrases().iter().enumerate()
            .filter_map(|(channel_index, phrase_option)| {
                phrase_option.and_then(|phrase_index| {
                    Some(channels[channel_index].phrases[phrase_index as usize].length())
                })
            })
            .max()
            // When nothing is playing, we still need some kind of length to calculate when to queue next sequence
            .or(Some(Phrase::default_length()))
            .unwrap()
    }
}
