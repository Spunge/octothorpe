
use std::ops::Range;

use super::note::*;
use super::playable::Playable;
use super::cycle::Cycle;
use super::events::*;
use super::TimebaseHandler;

#[derive(Debug, Clone)]
pub struct PlayedPattern {
    // Index in instruments patterns array
    pub index: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

pub struct PlayingPattern {
    pub instrument: usize,
    pub pattern: usize,
    // Start & end in ticks
    pub start: u32,
    pub end: u32,
}

#[derive(Clone)]
pub struct Pattern {
    note_events: Vec<NoteEvent>,

    pub playable: Playable,
    channel: u8,
    pub notes: Vec<Note>,
    pub base_note: u8,
    pub is_recording: bool,
}

impl Pattern {
    const BASE_NOTE: u8 = 49;

    fn minimum_length() -> u32 { TimebaseHandler::TICKS_PER_BEAT * 4 }

    fn create(channel: u8, notes: Vec<Note>) -> Self {
        Pattern {
            note_events: vec![],

            playable: Playable::new(TimebaseHandler::bars_to_ticks(1), TimebaseHandler::bars_to_ticks(1), 1, 5),
            channel,
            notes,
            // TODO - Use scales for this
            // Put a4 in center of grid
            base_note: Self::BASE_NOTE,
            is_recording: false,
        }
    }

    pub fn new(channel: u8) -> Self {
        Pattern::create(channel, vec![])
    }
    
    pub fn clear_note_events(&mut self) {
        self.note_events = vec![];
    }

    pub fn length(&self) -> u32 {
        let max_note = self.note_events.iter()
            .max_by_key(|event| event.start);

        let min = Self::minimum_length();

        if let Some(note) = max_note { 
            (note.start / min + 1) * min
        } else { 
            min
        }
    }

    pub fn add_note_start(&mut self, tick: u32, note: u8, velocity: u8) {
        let previous = self.note_events.iter()
            .filter(|event| event.note == note).last();

        if let Some(NoteEvent { stop: None, .. }) = previous {
            return;
        }

        self.note_events.push(NoteEvent::new(tick, note, velocity));
    }

    pub fn add_note_stop(&mut self, tick: u32, note: u8, velocity: u8) {
         // What note event is this stop for?
        let index = self.note_events.iter_mut().enumerate()
            .filter(|(_, event)| event.note == note).last().unwrap().0;
        
        let length = self.length();
        println!("{:?}", length);

        // Get event from events so we can compare others
        let mut event = self.note_events.swap_remove(index);
        event.stop = Some(tick);

        // Remove events that are contained in current event
        self.note_events.retain(|other| {
            event.note != other.note || ! event.contains(other, length)
        });

        // Resize events around new event, add new event when previous event is split by current event
        let mut split_events: Vec<NoteEvent> = self.note_events.iter_mut()
            .filter(|other| event.note == other.note)
            // Is event split by current event?
            // Create 2 events for events that are split by current event
            .filter_map(|other| other.resize_to_fit(&event, length))
            .collect();

        self.note_events.append(&mut split_events);
        self.note_events.push(event);
    }

    pub fn led_states(&mut self) -> Vec<(i32, i32, u8)> {
        let coords = self.notes.iter()
            // start, end, y
            .map(|note| (note.start, note.end, self.base_note as i32 - note.key as i32))
            .collect();

        self.playable.led_states(coords)
    }

    pub fn reset(&mut self) {
        self.base_note = Self::BASE_NOTE;
        self.notes = vec![];
    }

    pub fn change_base_note(&mut self, delta: i32) {
        let base_note = self.base_note as i32 + delta;

        // 21 is A0
        if base_note >= 25 && base_note <= 127 {
            self.base_note = base_note as u8;
        }
    }

    // Start recording notes from input into pattern
    pub fn switch_recording_state(&mut self) {
        self.is_recording = ! self.is_recording;
    }

    /*
    pub fn change_length(&mut self, length_modifier: u32) {
        let current_modifier = self.playable.length_modifier();
        let current_length = self.playable.length;

        if let Some(next_modifier) = self.playable.change_length(length_modifier) {
            // Add to current patterns
            if current_modifier < next_modifier {
                let times = next_modifier / current_modifier;

                let notes: Vec<Note> = (1..times).into_iter()
                    .flat_map(|multiplier| -> Vec<Note> {
                        self.notes.iter()
                            .map(|note| note.clone())
                            .map(|mut note| { 
                                note.start = note.start + multiplier * current_length;
                                note.end = note.end + multiplier * current_length;
                                note
                            })
                            .collect()
                    })
                    .collect();

                self.notes.extend(notes);
            } 

            // Cut from current patterns
            if current_modifier > next_modifier {
                let new_length = next_modifier * self.playable.minimum_length;

                self.notes.retain(|note| {
                    note.start < new_length
                });

                self.notes.iter_mut().for_each(|note| {
                    if note.end > new_length {
                        note.end = new_length;
                    }
                });
            }
        }
    }
    */

    pub fn quantize(&self, tick: u32, quantize_level: u8) -> u32 {
        let quantize_by_ticks = TimebaseHandler::beats_to_ticks(1.0) / quantize_level as u32;
        let offset = tick % quantize_by_ticks;
    
        if offset < quantize_by_ticks / 2 {
            tick - offset
        } else {
            (tick - offset) + quantize_by_ticks
        }
    }

    pub fn playing_notes(&self, cycle: &Cycle, start: u32, end: u32) -> Vec<(u32, &Note)> {
         self.notes.iter()
            .filter_map(move |note| {
                let note_start = note.start + start;

                // Does note fall in cycle?
                if note_start >= cycle.start && note_start < cycle.end && note_start < end {
                    let delta_ticks = note_start - cycle.start;

                    Some((delta_ticks, note))
                } else {
                    None
                }
            })
            .collect()
    }
}

