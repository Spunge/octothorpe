
use crate::*;

pub struct OLD_APC20 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    device_id: u8,
    local_id: u8,

    cue_knob: CueKnob,
    master: Single,

    // Lights
    grid: Grid,
    side: Side,
    indicator: WideRow,
    channel: WideRow,
    activator: WideRow,
    solo: WideRow,
    //arm: WideRow,
}

impl APC_trait for OLD_APC20 {
    type Loopable = Phrase;

    const CHANNEL_OFFSET: u8 = 0;
    const HEAD_COLOR: u8 = 3;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn loopable_ticks_per_button(&self, surface: &Surface) -> u32 { surface.phrase_ticks_per_button() }
    fn loopable_ticks_in_grid(&self, surface: &Surface) -> u32 { surface.phrase_ticks_in_grid() }
    fn loopable_zoom_level(&self, surface: &Surface) -> u8 { surface.phrase_zoom_level() }
    fn set_loopable_zoom_level(&self, sequencer: &Sequencer, surface: &mut Surface, zoom_level: u8) { 
        surface.set_phrase_zoom_level(sequencer, zoom_level);
    }
    fn shown_loopable_offset(&self, surface: &Surface) -> u32 { surface.phrase_offset(surface.channel_shown()) }
    fn set_shown_loopable_offset(&self, sequencer: &Sequencer, surface: &mut Surface, offset: u32) {
        surface.set_phrase_offset(sequencer, surface.channel_shown(), offset);
    }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &Surface) -> u8 { surface.phrase_shown(surface.channel_shown()) }

    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, surface: &Surface) -> &'a Self::Loopable { 
        let channel = sequencer.channel(surface.channel_shown());
        channel.phrase(self.shown_loopable_index(surface))
    }
    fn shown_loopable_mut<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let channel = sequencer.channel_mut(surface.channel_shown());
        channel.phrase_mut(self.shown_loopable_index(surface))
    }

    // Get indexes of currently playing phrases in showed channel
    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .map(|(_, _, phrase_index)| phrase_index)
            .collect()
    }

    fn playing_loopable_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases for currently selected channel
        let shown_phrase_index = self.shown_loopable_index(surface);
        let length = sequencer.channel(surface.channel_shown()).phrase(shown_phrase_index).length();

        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .filter(|(_, _, index)| *index == shown_phrase_index)
            .map(|(range, sequence_start, _)| {
                let iterations = (range.start - sequence_start) / length;

                (range, sequence_start + iterations * length)
            })
            .collect()
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn master(&mut self) -> &mut Single { &mut self.master }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn side(&mut self) -> &mut Side { &mut self.side }
    fn channel(&mut self) -> &mut WideRow { &mut self.channel }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("apc20_in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("apc20_out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            channel: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            //arm: WideRow::new(0x30),
        }
    }

    fn process_inputevent(&mut self, event: &InputEvent, _cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) {
        let channel = sequencer.channel_mut(surface.channel_shown());
        let phrase = channel.phrase_mut(surface.phrase_shown(surface.channel_shown()));

        // Only process channel note messages
        match event.event_type {
            // TODO - Use indicator row as fast movement
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CHANNEL_OFFSET, button_type);

                match surface.view {
                    View::Channel => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let offset = surface.phrase_offset(surface.channel_shown());
                                // We draw grids from bottom to top
                                let ticks_per_button = self.loopable_ticks_per_button(surface);

                                if let Some(tick_range) = self.should_add_event(phrase, modifier, ticks_per_button, x, y, offset, y) {
                                    phrase.try_add_starting_event(LoopablePatternEvent::new(tick_range.start, y));
                                    let mut event = phrase.get_last_event_on_row(y);
                                    event.set_stop(tick_range.stop);

                                    phrase.add_complete_event(event);
                                }
                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                if let Some(ButtonType::Side(modifier_index)) = modifier {
                                    channel.clone_phrase(modifier_index, index);
                                } else if let Some(ButtonPress { button_type: ButtonType::Shift, .. }) = global_modifier {
                                    channel.phrase_mut(index).clear_events();
                                } else {
                                    surface.show_phrase(surface.channel_shown(), index);
                                }
                            },
                            ButtonType::Activator(index) => {
                                phrase.set_length(Phrase::default_length() * (index as u32 + 1));
                            },
                            _ => (),
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    // Draw APC specific things
    fn draw(&mut self, sequencer: &mut Sequencer, surface: &mut Surface) {
        match surface.view {
            View::Channel => {
                let loopable = self.shown_loopable(sequencer, surface);

                // Draw main grid
                let events = loopable.events().iter();
                self.draw_loopable_events(events, surface.phrase_offset(surface.channel_shown()), 0, self.loopable_ticks_in_grid(surface), Self::HEAD_COLOR, Self::TAIL_COLOR);

                // Length selector
                for index in 0 .. (loopable.length() / Self::Loopable::default_length()) {
                    self.activator.draw(index as u8, 1);
                }
            },
            View::Sequence => {
            },
            View::Timeline => {
            }
        }
    }
}
