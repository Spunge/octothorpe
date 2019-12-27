
#[derive(PartialEq)]
pub enum View {
    Instrument,
    Sequence,
}

pub enum Visualization {
    PatternGrid,
    PatternIndicator,
    PatternLength,
    PatternZoom,
    PatternSelect(u32),
    PhraseGrid,
    PhraseIndicator,
    PhraseLength,
    PhraseZoom,
    PhraseSelect(u32),
    QuantizationLevel,
    QuantizationState,
    SequenceGrid,
    SequenceSelect(u32),
    InstrumentSelect,
}

pub struct Surface {
    pub view: View,
    redraw: Vec<Visualization>,

    instrument_shown: usize,
    sequence_shown: usize,

    patterns_shown: [usize; 16],
    phrases_shown: [usize; 16],
}

impl Surface {
    pub fn new() -> Self {
        Surface { 
            view: View::Instrument, 
            redraw: vec![],

            instrument_shown: 0,
            sequence_shown: 0,

            patterns_shown: [0; 16],
            phrases_shown: [0; 16],
        }
    }

    pub fn switch_view(&mut self) { 
        self.view = match self.view {
            View::Instrument => View::Sequence,
            // TODO When switching from sequence to instrument, don't note_off the instrument grid
            // Clear as we do not want the selected instrument grid to clear
            //self.indicator_note_offs = vec![];
            View::Sequence => View::Instrument,
        }
    }

    pub fn show_instrument(&mut self, index: u8) { self.instrument_shown = index as usize; }
    pub fn instrument_shown(&self) -> usize { self.instrument_shown }

    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index as usize; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown }

    pub fn show_phrase(&mut self, index: u8) { self.phrases_shown[self.instrument_shown()] = index as usize; }
    pub fn phrase_shown(&self) -> usize { self.phrases_shown[self.instrument_shown()] }

    pub fn show_pattern(&mut self, index: u8) { self.patterns_shown[self.instrument_shown()] = index as usize; }
    pub fn pattern_shown(&self) -> usize { self.patterns_shown[self.instrument_shown()] }

    pub fn toggle_instrument(&mut self, index: u8) {
        // If we click selected instrument, return to sequence for peeking
        if self.instrument_shown() == index as usize {
            self.switch_view();
        } else {
            // Otherwise select instrument && switch
            self.show_instrument(index);
            // TODO - What does instrument target? Move this to "instrument"
            //self.keyboard_target = instrument;
            //self.drumpad_target = instrument;

            if let View::Sequence = self.view { self.switch_view() }
        }
    }

    pub fn toggle_sequence(&mut self, index: u8) {
        // When we press currently selected overview, return to instrument view, so we can peek
        if self.sequence_shown() == index as usize {
            self.switch_view();
        } else {
            // If we select a new sequence, show that
            self.show_sequence(index);

            if let View::Instrument = self.view { self.switch_view() }
        }
    }
}
