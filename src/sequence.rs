
use super::instrument::Instrument;

pub struct Sequence {
    // Phrase that's playing for instrument, array index = instrument
    phrases: [Option<u8>; 16],
    pub active: [bool; 16],

    pub knob_group: u8,
    knob_values: [u8; 128],
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            phrases: [Some(0); 16],
            active: [true; 16],

            knob_group: 0,
            knob_values: [0; 128],
        }
    }

    /*
    pub fn led_states(&mut self, group: u8) -> Vec<(i32, i32, u8)> {
        let start = 8 * group;
        let end = start + 8;

        self.phrases[start as usize .. end as usize].iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                (instrument as i32, phrase.unwrap() as i32, 1)
            })
            .collect()
    }

    pub fn phrases<'a>(&'a self) -> impl Iterator<Item=(usize, usize)> + 'a {
        self.phrases.iter()
            .enumerate()
            .filter(|(_, phrase)| phrase.is_some())
            .map(|(instrument, phrase)| {
                (instrument, phrase.unwrap())
            })
    }

    // Get length in ticks of sequence based on the longest phrase it's playing
    pub fn length(&self, instruments: &[Instrument]) -> Option<u32> {
        self.phrases()
            .map(|(instrument, phrase)| {
                instruments[instrument].phrases[phrase].playable.length
            })
            .max()
    }
    */

    pub fn active_phrase(&self, index: usize) -> Option<u8> {
        self.phrases[index].and_then(|phrase| if self.active[index] { Some(phrase) } else { None })
    }

    pub fn toggle_row(&mut self, phrase: u8) {
        self.phrases = [Some(phrase); 16];
    }

    pub fn toggle_phrase(&mut self, instrument: u8, phrase: u8) {
        self.phrases[instrument as usize] = self.phrases[instrument as usize]
            // Select different phrase or disable phrase
            .and_then(|previous| if previous == phrase { None } else { Some(phrase) })
            .or(Some(phrase))
    }

    pub fn toggle_active(&mut self, instrument: u8) {
        self.active[instrument as usize] = ! self.active[instrument as usize];
    }

    /*
    // Get playing phrases of this sequence
    pub fn playing_phrases(&self, instruments: &[Instrument], sequence_start: u32) -> Vec<PlayingPhrase> {
        // Could be this is a 0 length sequence
        if let Some(sequence_length) = self.length(instruments) {
            self.phrases()
                .filter(|(instrument, _)| self.active[*instrument])
                .flat_map(|(instrument, phrase)| {
                    let phrase_length = instruments[instrument].phrases[phrase].playable.length;

                    (0..sequence_length)
                        .step_by(phrase_length as usize)
                        .into_iter()
                        .map(move |ticks| {
                            let start = sequence_start + ticks;
                            let end = start + phrase_length;
                            PlayingPhrase { instrument, phrase, start, end }
                        })
                })
                .collect()
        } else {
            vec![]
        }
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
    */
}
