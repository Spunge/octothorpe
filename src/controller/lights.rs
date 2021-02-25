
use std::mem;
use super::super::message::*;

// TODO - We could probably macro these grids, but.. alas, i'm not familiar enough with macros

pub trait Drawable {
    fn output_messages(&mut self, frame: u32) -> Vec<TimedMessage> {
        self.output().into_iter()
            .map(|(channel, note, velocity)| TimedMessage::new(frame, Message::Note([channel, note, velocity])))
            .collect()
    }

    fn output(&mut self) -> Vec<(u8, u8, u8)>;

    fn reset(&mut self);
}

pub struct Coordinate {
    pub x: u8,
    pub y: u8,
}

impl Coordinate {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LedState {
    Off,
    Green,
    Orange,
    Red,
}

/*
 * Grid of buttons that will remember button states so we'll only have to send midi messages when a
 * button state changes for buttons with lights
 */
pub struct IlluminatedButtonGrid {
    width: u8,
    height: u8,

    state: Vec<LedState>,
    next_state: Vec<LedState>,
}

impl IlluminatedButtonGrid {
    pub fn new(width: u8, height: u8) -> Self {
        Self { 
            width,
            height,
            state: vec![LedState::Off; Self::buffer_length(width, height)],
            next_state: vec![LedState::Off; Self::buffer_length(width, height)],
        }
    }

    // Get length of buffer based on width & height
    fn buffer_length(width: u8, height: u8) -> usize {
        width as usize * height as usize
    }

    // Get the position in the buffer array from coordinate
    fn coordinate_to_buffer_index(coordinate: Coordinate) -> usize {
        coordinate.y as usize * 8 + coordinate.x as usize
    }

    // Get coordinate from buffer index
    fn buffer_index_to_coordinate(&self, index: usize) -> Coordinate {
        Coordinate {
            x: (index % self.height as usize) as u8,
            y: (index / self.width as usize) as u8,
        }
    }

    // Check if a coordinate falls within the grid
    fn contains(&self, coordinate: Coordinate) -> bool {
        Self::coordinate_to_buffer_index(coordinate) < self.state.len()
    }

    // Set state for next draw
    pub fn set_next_state(&mut self, coordinate: Coordinate, state: LedState) {
        self.next_state[Self::coordinate_to_buffer_index(coordinate)] = state;
    }

    // Get all coordinates that changed value
    pub fn changed_state(&mut self) -> Vec<(Coordinate, LedState)> {
        // Meeeeeh, rust array comparison works up to 32 elements...
        // https://doc.rust-lang.org/std/primitive.array.html#impl-PartialEq%3C%5BB%3B%20N%5D%3E

        // Create array of changed buttonstates
        let changed = self.state.iter().enumerate()
            // We only want to return changed state
            .filter(|(index, _)| self.state[*index] != self.next_state[*index])
            // Return a buttonstate for every changed state in buffer
            .map(|(index, value)| (self.buffer_index_to_coordinate(index), self.next_state[index]))
            .collect();

        mem::swap(&mut self.state, &mut self.next_state);

        self.next_state = vec![LedState::Off; Self::buffer_length(self.width, self.height)];

        // Return changed button states
        changed
    }
}

// 40 boi at the top
pub struct Grid {
    state: [u8; 40],
    next_state: [u8; 40],
}

// 5 high grid
pub struct Side {
    state: [u8; 5],
    next_state: [u8; 5],
}

// 8 wide grid
pub struct WideRow {
    state: [u8; 8],
    next_state: [u8; 8],

    note: u8,
}

// 4 wide grid
/*
pub struct NarrowRow {
    state: [u8; 4],
    next_state: [u8; 4],

    note: u8,
}
*/

pub struct Single {
    state: u8,
    next_state: u8,
    
    note: u8,
}

impl Grid {
    pub fn new() -> Self {
        // 9 does not exist, this way we force redraw of *all* leds first run
        Grid { state: [9; 40], next_state: [0; 40] }
    }

    pub fn width(&self) -> u8 { 8 }
    pub fn height(&self) -> u8 { 5 }

    fn index(x: u8, y: u8) -> usize {
        y as usize * 8 + x as usize
    }

    pub fn try_draw(&mut self, x: i32, y: u8, value: u8) {
        if x >= 0 {
            self.draw(x as u8, y, value);
        }
    }

    pub fn draw(&mut self, x: u8, y: u8, value: u8) {
        if x < self.width() && y < self.height() {
            // 4 - as grid & side are flipped upside down to make MIDI notes go up..
            self.next_state[Self::index(x, 4 - y)] = value;
        }
    }
}

impl Drawable for Grid {
    fn reset(&mut self) {
        self.state = [9; 40];
    }

    fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        // Meeeeeh, rust array comparison works up to 32 elements...
        // https://doc.rust-lang.org/std/primitive.array.html#impl-PartialEq%3C%5BB%3B%20N%5D%3E
        if self.next_state[0..20] != self.state[0..20] || self.next_state[20..] != self.state[20..]  {
            for x in 0 .. self.width() {
                for y in 0 .. self.height() {
                    let index = Self::index(x, y);

                    if self.next_state[index] != self.state[index] {
                        let channel = x as u8 + if self.next_state[index] > 0 { 0x90 } else { 0x80 };
                        let note = 0x35 + y as u8;

                        output.push((channel, note, self.next_state[index]));
                    }
                }
            }
        }

        self.state = self.next_state;
        self.next_state = [0; 40];
        output
    }
}

impl Side {
    pub fn new() -> Self {
        Side { state: [9; 5], next_state: [0; 5] }
    }

    pub fn height(&self) -> u8 { 5 }

    pub fn draw(&mut self, index: u8, value: u8) {
        if index < self.height() {
            // 4 - as grid & side are flipped upside down to make MIDI notes go up..
            self.next_state[4 - index as usize] = value;
        }
    }
}

impl Drawable for Side {
    fn reset(&mut self) {
        self.state = [9; 5];
    }

    fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        if self.next_state != self.state {
            for index in 0 .. self.height() as usize {
                if self.next_state[index] != self.state[index] {
                    let channel = if self.next_state[index] == 1 { 0x90 } else { 0x80 };
                    let note = 0x52 + index as u8;

                    output.push((channel, note, self.next_state[index]));
                }
            }
        }

        self.state = self.next_state;
        self.next_state = [0; 5];
        output
    }
}

impl WideRow {
    pub fn new(note: u8) -> Self {
        WideRow { state: [9; 8], next_state: [0; 8], note, }
    }

    pub fn width(&self) -> u8 { 8 }

    pub fn draw(&mut self, index: u8, value: u8) {
        if index < self.width() {
            self.next_state[index as usize] = value;
        }
    }
}

impl Drawable for WideRow {
    fn reset(&mut self) {
        self.state = [9; 8];
    }

    fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        if self.next_state != self.state {
            for index in 0 .. self.width() as usize {
                if self.next_state[index] != self.state[index] {
                    let channel = if self.next_state[index] == 1 { 0x90 } else { 0x80 };

                    output.push((channel + index as u8, self.note, self.next_state[index]));
                }
            }
        }

        self.state = self.next_state;
        self.next_state = [0; 8];
        output
    }
}

/*
impl NarrowRow {
    pub fn new(note: u8) -> Self {
        NarrowRow { state: [9; 4], next_state: [0; 4], note, }
    }

    pub fn width(&self) -> u8 { 4 }

    pub fn draw(&mut self, index: u8, value: u8) {
        if index < self.width() {
            self.next_state[index as usize] = value;
        }
    }
}
*/

impl Single {
    pub fn new(note: u8) -> Self {
        Single { state: 9, next_state: 0, note, }
    }

    pub fn draw(&mut self, value: u8) {
        self.next_state = value;
    }
}

impl Drawable for Single {
    fn reset(&mut self) {
        self.state = 9;
    }

    fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        if self.next_state != self.state {
            let channel = if self.next_state == 1 { 0x90 } else { 0x80 };
            output.push((channel, self.note, self.next_state));
        }

        self.state = self.next_state;
        self.next_state = 0;
        output
    }
}
