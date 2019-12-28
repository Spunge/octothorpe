

// TODO - We could probably macro these grids, but.. alas, i'm not familiar enough with macros

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
pub struct NarrowRow {
    state: [u8; 4],
    next_state: [u8; 4],

    note: u8,
}

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

    pub fn width(&self) -> usize { 8 }
    pub fn height(&self) -> usize { 5 }

    fn index(x: usize, y: usize) -> usize {
        y * 8 + x
    }

    pub fn draw(&mut self, x: usize, y: usize, value: u8) {
        if x < self.width() && y < self.height() {
            self.next_state[Self::index(x, y)] = value;
        }
    }

    pub fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        // Meeeeeh, rust array comparison works up to 32 elements...
        // https://doc.rust-lang.org/std/primitive.array.html#impl-PartialEq%3C%5BB%3B%20N%5D%3E
        if self.next_state[1..20] != self.state[1..20] || self.next_state[20..] != self.state[20..]  {
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

    pub fn height(&self) -> usize { 5 }

    pub fn draw(&mut self, index: usize, value: u8) {
        if index < self.height() {
            self.next_state[index] = value;
        }
    }

    pub fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        if self.next_state != self.state {
            for index in 0 .. self.height() {
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

    pub fn width(&self) -> usize { 8 }

    pub fn draw(&mut self, index: usize, value: u8) {
        if index < self.width() {
            self.next_state[index] = value;
        }
    }

    pub fn output(&mut self) -> Vec<(u8, u8, u8)> {
        let mut output = vec![];

        if self.next_state != self.state {
            for index in 0 .. self.width() {
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

impl NarrowRow {
    pub fn new(note: u8) -> Self {
        NarrowRow { state: [9; 4], next_state: [0; 4], note, }
    }

    pub fn width(&self) -> usize { 4 }

    pub fn draw(&mut self, index: usize, value: u8) {
        if index < self.width() {
            self.next_state[index] = value;
        }
    }
}

impl Single {
    pub fn new(note: u8) -> Self {
        Single { state: 9, next_state: 0, note, }
    }

    pub fn draw(&mut self, value: u8) {
        self.next_state = value;
    }
}
