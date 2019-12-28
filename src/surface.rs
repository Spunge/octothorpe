
#[derive(PartialEq)]
pub enum View {
    Instrument,
    Sequence,
}

pub struct Surface {
    pub view: View,

    instrument_shown: u8,
    sequence_shown: u8,
}

impl Surface {
    pub fn new() -> Self {
        Surface { 
            view: View::Instrument, 

            instrument_shown: 0,
            sequence_shown: 0,
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

    pub fn show_instrument(&mut self, index: u8) { self.instrument_shown = index; }
    pub fn instrument_shown(&self) -> usize { self.instrument_shown as usize }

    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }

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
