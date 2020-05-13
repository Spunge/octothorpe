
use super::controller::*;
use super::controller::input::*;
use super::TimebaseHandler;
use super::Sequencer;
use super::loopable::*;
use super::cycle::*;
use super::mixer::*;
use super::memory::*;
use super::display::*;

#[derive(Debug, PartialEq)]
pub enum View {
    Track,
    Sequence,
    //Timeline,
}
pub enum TrackView {
    Split,
    Pattern,
    Phrase,
    Timeline,
}


pub struct Surface {
    pub view: View,
    pub track_view: TrackView,

    track_shown: u8,
    sequence_shown: u8,

    pub pattern_display: PatternDisplay,
    pub phrase_display: PhraseDisplay,
    pub timeline_display: TimelineDisplay,
}

impl Surface {
    pub const PATTERN_TICKS_PER_BUTTON: u32 = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
    pub const PHRASE_TICKS_PER_BUTTON: u32 = Self::PATTERN_TICKS_PER_BUTTON * 4;
    pub const TIMELINE_TICKS_PER_BUTTON: u32 = Self::PHRASE_TICKS_PER_BUTTON * 1;

    pub fn new() -> Self {
        let pattern_button_ticks = TimebaseHandler::TICKS_PER_BEAT as u32 * 2;
        let phrase_button_ticks = pattern_button_ticks * 4;
        let timeline_button_ticks= phrase_button_ticks * 4;

        Surface { 
            view: View::Track, 
            track_view: TrackView::Split,

            track_shown: 0,
            sequence_shown: 0,

            pattern_display: PatternDisplay::new(pattern_button_ticks),
            phrase_display: PhraseDisplay::new(phrase_button_ticks),
            timeline_display: TimelineDisplay::new(timeline_button_ticks),
        }
    }

    pub fn switch_view(&mut self, view: View) { 
        self.view = view;
    }

    pub fn show_track(&mut self, index: u8) { self.track_shown = index; }
    pub fn track_shown(&self) -> usize { self.track_shown as usize }
    pub fn show_sequence(&mut self, index: u8) { self.sequence_shown = index; }
    pub fn sequence_shown(&self) -> usize { self.sequence_shown as usize }

    pub fn process_midi_input(&mut self, cycle: &ProcessCycle, controllers: &mut Vec<APC>, memory: &mut Memory, sequencer: &mut Sequencer, mixer: &mut Mixer) {
        controllers.iter_mut()
            .flat_map(|controller| controller.input_events(cycle))
            .for_each(|event| {
                // Process inputs that are same over all views
                match event.event_type {
                    InputEventType::FaderMoved { value, fader_type: FaderType::Track(index) } => {
                        mixer.fader_adjusted(event.time, index, value);
                    },
                    // Make sure that button memory stays up - to - date
                    InputEventType::ButtonPressed(button_type) => memory.buttons.press(button_type),
                    InputEventType::ButtonReleased(button_type) => memory.buttons.release(button_type),
                    _ => (),
                };

                // Process view specific controls
                match self.view {
                    View::Track => {
                        let track_index = self.track_shown();
                        let track = sequencer.track_mut(track_index);

                        match self.track_view {
                            TrackView::Split => {
                                self.phrase_display.process_inputevent(&event, 0, 8, memory, track, track_index);
                                self.pattern_display.process_inputevent(&event, 8, 8, memory, track, track_index);
                            },
                            TrackView::Pattern => {
                                self.pattern_display.process_inputevent(&event, 0, 16, memory, track, track_index);
                            },
                            TrackView::Phrase => {
                                self.phrase_display.process_inputevent(&event, 0, 16, memory, track, track_index);
                            },
                            TrackView::Timeline => {
                                self.timeline_display.process_inputevent(&event, 0, 16, memory, track, track_index);
                            },
                        };
                    },
                    View::Sequence => {
                
                    }
                }
            });
    }

    pub fn output_midi(&mut self, cycle: &ProcessCycle, controllers: &Vec<APC>, sequencer: &Sequencer) {
    
    }
}
