
use crate::*;

pub struct OLD_APC40 {
    // Ports that connect to APC
    input: jack::Port<jack::MidiIn>,
    output: MidiOut,

    identified_cycles: u8,
    device_id: u8,
    local_id: u8,
    //knob_offset: u8,

    cue_knob: CueKnob,
    master: Single,

    grid: Grid,
    side: Side,
    indicator: WideRow,
    channel: WideRow,
    activator: WideRow,
    solo: WideRow,
    //arm: WideRow,
}

impl APC_trait for OLD_APC40 {
    type Loopable = Pattern;

    const CHANNEL_OFFSET: u8 = 8;
    const HEAD_COLOR: u8 = 1;
    const TAIL_COLOR: u8 = 5;

    fn identified_cycles(&self) -> u8 { self.identified_cycles }
    fn set_identified_cycles(&mut self, cycles: u8) { self.identified_cycles = cycles }
    fn local_id(&self) -> u8 { self.local_id }
    fn set_local_id(&mut self, local_id: u8) { self.local_id = local_id }
    fn device_id(&self) -> u8 { self.device_id }
    fn set_device_id(&mut self, device_id: u8) { self.device_id = device_id }

    fn loopable_ticks_per_button(&self, surface: &Surface) -> u32 { surface.pattern_ticks_per_button() }
    fn loopable_ticks_in_grid(&self, surface: &Surface) -> u32 { surface.pattern_ticks_in_grid() }
    fn loopable_zoom_level(&self, surface: &Surface) -> u8 { surface.pattern_zoom_level() }
    fn set_loopable_zoom_level(&self, sequencer: &Sequencer, surface: &mut Surface, zoom_level: u8) { 
        surface.set_pattern_zoom_level(sequencer, zoom_level);
    }
    fn shown_loopable_offset(&self, surface: &Surface) -> u32 { surface.pattern_offset(surface.channel_shown()) }
    fn set_shown_loopable_offset(&self, sequencer: &Sequencer, surface: &mut Surface, offset: u32) { 
        surface.set_pattern_offset(sequencer, surface.channel_shown(), offset) 
    }

    fn output(&mut self) -> &mut MidiOut { &mut self.output }
    fn input(&self) -> &jack::Port<jack::MidiIn> { &self.input }

    fn shown_loopable_index(&self, surface: &Surface) -> u8 { surface.pattern_shown(surface.channel_shown()) }

    fn shown_loopable<'a>(&self, sequencer: &'a Sequencer, surface: &Surface) -> &'a Self::Loopable { 
        let channel = sequencer.channel(surface.channel_shown());
        channel.pattern(self.shown_loopable_index(surface))
    }
    fn shown_loopable_mut<'a>(&self, sequencer: &'a mut Sequencer, surface: &mut Surface) -> &'a mut Self::Loopable { 
        let channel = sequencer.channel_mut(surface.channel_shown());
        channel.pattern_mut(self.shown_loopable_index(surface))
    }

    fn playing_loopable_indexes(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<u8> {
        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.channel_shown(), phrase_index, sequence_start).into_iter()
                    .map(|(pattern_index, _, _, _, _)| pattern_index)
            })
            .collect()
    }

    fn playing_loopable_ranges(&self, cycle: &ProcessCycle, sequencer: &Sequencer, surface: &mut Surface) -> Vec<(TickRange, u32)> {
        // Get playing phrases of currently selected channel
        let shown_pattern_index = self.shown_loopable_index(surface);
        let pattern = sequencer.channel(surface.channel_shown()).pattern(shown_pattern_index);

        sequencer.playing_phrases(surface.channel_shown(), &cycle.tick_range).into_iter()
            .flat_map(|(tick_range, sequence_start, phrase_index)| {
                sequencer.playing_patterns(&tick_range, surface.channel_shown(), phrase_index, sequence_start).into_iter()
                    .filter(|(pattern_index, _, _, _, _)| *pattern_index == shown_pattern_index)
                    .map(move |(_, absolute_start, relative_range, _, _)| {
                        let absolute_range = relative_range.plus(absolute_start);

                        // Make sure indicator loops around when pattern has explicit length
                        let start = if pattern.has_explicit_length() {
                            let length = pattern.length();
                            let iterations = relative_range.start / length;
                            absolute_start + iterations * length
                        } else {
                            absolute_start
                        };

                        (absolute_range, start)
                    })
            })
            .collect()
    }

    fn cue_knob(&mut self) -> &mut CueKnob { &mut self.cue_knob }
    fn master(&mut self) -> &mut Single { &mut self.master }
    fn grid(&mut self) -> &mut Grid { &mut self.grid }
    fn side(&mut self) -> &mut Side { &mut self.side }
    fn channel(&mut self) -> &mut WideRow { &mut self.channel }
    fn indicator(&mut self) -> &mut WideRow { &mut self.indicator }
    fn activator(&mut self) -> &mut WideRow { &mut self.activator }
    fn solo(&mut self) -> &mut WideRow { &mut self.solo }

    fn new(client: &jack::Client) -> Self {
        let input = client.register_port("apc40_in", jack::MidiIn::default()).unwrap();
        let output = client.register_port("apc40_out", jack::MidiOut::default()).unwrap();
        
        Self {
            input,
            output: MidiOut::new(output),

            identified_cycles: 0,
            local_id: 0,
            device_id: 0,
            // Offset knobs by this value to support multiple groups
            //knob_offset: 0,

            cue_knob: CueKnob::new(),
            master: Single::new(0x50),

            grid: Grid::new(),
            side: Side::new(),
            indicator: WideRow::new(0x34),
            channel: WideRow::new(0x33),
            activator: WideRow::new(0x32),
            solo: WideRow::new(0x31),
            // TODO - Put length indicator here, get length from longest LoopablePatternEvent in phrases?
            //arm: WideRow::new(0x30),
        }
    }

    /*
     * Process APC40 specific midi input, shared input is handled by APC trait
     */
    fn process_inputevent(&mut self, event: &InputEvent, cycle: &ProcessCycle, sequencer: &mut Sequencer, surface: &mut Surface) {
        // Only process channel note messages
        match event.event_type {
            // TODO - As we are not using a software mixer anymore, we don't have a master fader
            //InputEventType::FaderMoved { value, fader_type: FaderType::Master } => {
                //mixer.master_adjusted(event.time, value);
            //},
            // We use the crossfader to traverse patterns & phrases horizontally
            InputEventType::FaderMoved { value, fader_type: FaderType::CrossFade } => {
                let factor = value as f64 / 127.0;
                //let max_offset = self.max_offset(self.shown_loopable(sequencer, surface).length());
                
                //let offset = (max_offset as f64 * factor) as u32;
                surface.set_offsets_by_factor(sequencer, surface.channel_shown(), factor);
                //println!("{:?} {:?}", offset, max_offset);
                
                //self.set_offset(surface.channel_shown(), offset);
                //mixer.master_adjusted(event.time, value);
            },
            InputEventType::KnobTurned { value: _, knob_type: KnobType::Control(_index) } => {
                // TODO 
                //sequencer.knob_turned(event.time, index + self.knob_offset, value);
            },
            InputEventType::ButtonPressed(button_type) => {
                // Get modifier (other currently pressed key)
                let modifier = surface.button_memory.modifier(Self::CHANNEL_OFFSET, button_type);

                match surface.view {
                    View::Channel => {
                        match button_type {
                            ButtonType::Grid(x, y) => {
                                let channel = sequencer.channel_mut(surface.channel_shown());
                                let pattern = channel.pattern_mut(surface.pattern_shown(surface.channel_shown()));

                                // We subtract y from 4 as we want lower notes to be lower on
                                // the grid, the grid counts from the top
                                let offset = surface.pattern_offset(surface.channel_shown());
                                // We put base note in center of grid
                                let note = surface.pattern_base_note(surface.channel_shown()) - 2 + y;
                                let ticks_per_button = self.loopable_ticks_per_button(surface);

                                if let Some(tick_range) = self.should_add_event(pattern, modifier, ticks_per_button, x, y, offset, note) {
                                    pattern.try_add_starting_event(LoopableNoteEvent::new(tick_range.start, note, 127));
                                    let mut event = pattern.get_last_event_on_row(note);
                                    event.set_stop(tick_range.stop);
                                    event.stop_velocity = Some(127);

                                    pattern.add_complete_event(event);
                                }
                            },
                            ButtonType::Side(index) => {
                                let global_modifier = surface.button_memory.global_modifier(button_type);

                                // TODO - double press logic && recording logic
                                if false {
                                    //channel.pattern_mut(index).switch_recording_state()
                                } else {
                                    if let Some(ButtonType::Side(modifier_index)) = modifier {
                                        let channel = sequencer.channel_mut(surface.channel_shown());
                                        channel.clone_pattern(modifier_index, index);
                                    } else if let Some(ButtonPress { button_type: ButtonType::Shift, .. }) = global_modifier {
                                        surface.set_pattern_offset(sequencer, surface.channel_shown(), 0);

                                        let channel = sequencer.channel_mut(surface.channel_shown());
                                        channel.pattern_mut(index).clear_events();
                                    } else {
                                        surface.show_pattern(surface.channel_shown(), index);
                                    }
                                }
                            },
                            ButtonType::Activator(index) => {
                                let channel = sequencer.channel_mut(surface.channel_shown());
                                let pattern = channel.pattern_mut(surface.pattern_shown(surface.channel_shown()));
                                let length = Pattern::minimum_length() * (index as u32 + 1);

                                if pattern.has_explicit_length() && pattern.length() == length {
                                    pattern.unset_length();
                                } else {
                                    pattern.set_length(length);
                                }
                            },
                            ButtonType::Up => {
                                let base_note = surface.pattern_base_note(surface.channel_shown());
                                surface.set_pattern_base_note(surface.channel_shown(), base_note + 4);
                            },
                            ButtonType::Down => {
                                let base_note = surface.pattern_base_note(surface.channel_shown());
                                surface.set_pattern_base_note(surface.channel_shown(), base_note - 4) 
                            },
                            ButtonType::Right => {
                                let ticks_per_button = self.loopable_ticks_per_button(surface);
                                let offset = surface.pattern_offset(surface.channel_shown());
                                // There's 8 buttons, shift view one gridwidth to the right
                                surface.set_pattern_offset(sequencer, surface.channel_shown(), offset + ticks_per_button * 8);
                            },
                            ButtonType::Left => {
                                let ticks_per_button = self.loopable_ticks_per_button(surface);
                                let offset = surface.pattern_offset(surface.channel_shown());
                                let new_offset = offset as i32 - (ticks_per_button * 8) as i32;
                                let offset = if new_offset >= 0 { new_offset as u32 } else { 0 };

                                surface.set_pattern_offset(sequencer, surface.channel_shown(), offset);
                            },
                            ButtonType::Quantization => {
                                // TODO - Move quantizing & quantize_level to "keyboard"
                                //sequencer.switch_quantizing();
                            },
                            _ => (),
                        }
                    },
                    _ => ()
                }

                match button_type {
                    ButtonType::Play => sequencer.start(cycle),
                    ButtonType::Stop => {
                        // Reset to 0 when we press stop button but we're already stopped
                        let (state, pos) = cycle.client.transport_query();
                        let is_transport_at_start = pos.bar == 1 && pos.beat == 1 && pos.tick == 0;
                        let global_modifier = surface.button_memory.global_modifier(button_type);

                        // Reset timeline when we shift press stop @ 0:0:0
                        if let (Some(ButtonPress { button_type: ButtonType::Shift, .. }), true) = (global_modifier, is_transport_at_start) {
                            sequencer.reset_timeline();
                        } else {
                            match state {
                                1 => sequencer.stop(cycle),
                                _ => {
                                    sequencer.reset(cycle);
                                    surface.set_timeline_offset(sequencer, 0);
                                },
                            };
                        }
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }

    fn draw(&mut self, sequencer: &mut Sequencer, surface: &mut Surface) {
        match surface.view {
            View::Channel => {
                let loopable = self.shown_loopable_mut(sequencer, surface);

                // Get base note of channel, as we draw the grid with base note in vertical center
                let base_note = surface.pattern_base_note(surface.channel_shown());
                let events = loopable.events().iter()
                    .filter(|event| event.note >= base_note - 2 && event.note <= base_note + 2);

                self.draw_loopable_events(events, surface.pattern_offset(surface.channel_shown()), base_note - 2, self.loopable_ticks_in_grid(surface), Self::HEAD_COLOR, Self::TAIL_COLOR);

                // pattern length selector
                if loopable.has_explicit_length() {
                    for index in 0 .. (loopable.length() / Self::Loopable::minimum_length()) {
                        self.activator.draw(index as u8, 1);
                    }
                }
            },
            View::Sequence => {
                // TODO - Draw sequence stuff
                // TODO - Output sequence indicator
            },
            View::Timeline => {
            
            }
        }
    }
}

