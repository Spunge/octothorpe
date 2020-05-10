
use super::track::Track;
use super::loopable::*;

pub struct Sequence {
    // Phrase that's playing for track, array index = track
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

    pub fn get_phrase(&self, track: usize) -> Option<u8> {
        self.phrases[track]
    }

    pub fn set_phrases(&mut self, phrase: u8) {
        self.phrases = [Some(phrase); 16];
    }

    pub fn set_phrase(&mut self, track: usize, phrase: u8) {
        self.phrases[track] = Some(phrase);
    }

    pub fn unset_phrase(&mut self, track: usize) {
        self.phrases[track] = None;
    }

    pub fn active_phrase(&self, track: usize) -> Option<u8> {
        self.phrases[track].and_then(|phrase| if self.active[track] { Some(phrase) } else { None })
    }

    pub fn toggle_active(&mut self, track: usize) {
        self.active[track as usize] = ! self.active[track as usize];
    }

    pub fn length(&self, tracks: &[Track]) -> u32 {
        self.phrases().iter().enumerate()
            .filter_map(|(track_index, phrase_option)| {
                phrase_option.and_then(|phrase_index| {
                    Some(tracks[track_index].phrases[phrase_index as usize].length())
                })
            })
            .max()
            // When nothing is playing, we still need some kind of length to calculate when to queue next sequence
            .or(Some(Phrase::minimum_length()))
            .unwrap()
    }
}
