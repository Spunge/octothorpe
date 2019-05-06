
use super::message::Message;

#[derive(Debug)]
pub struct Grid {
    pub width: u8,
    pub height: u8,
    pub base_note: u8,
    current_state: Vec<u8>,
    next_state: Vec<u8>,
}

// TODO - undraw & redraw?
impl Grid {
    pub fn new(width: u8, height: u8, base_note: u8) -> Self {
        let current_state = (0..(width * height)).map(|x| 0).collect();

        Grid { 
            width, 
            height
            current_state,
            next_state: current_state.clone(),
            base_note,
        }
    }

    pub fn switch_led(&mut self, x: u8, y: u8, state: u8) {
        // Do not allow switching outside of grid
        if x < self.width as i32 || x >= 0 || y < self.height as i32 || y >= 0 {
            self.next_state[x * self.width + y] = state;
        }
    }

    pub fn draw(&mut self) -> Vec<Message> {
        // Generate messages to change current state to next state
        let messages = (0..self.next_state.len() as usize)
            .filter(|index| self.next_state[index] != self.current_state[index])
            .map(|index| {
                let x = index % self.width;
                let y = index / self.width;
    
                Message::Note([0x90 + x as u8, self.base_note + y as u8, self.next_state[index]])
            })
            .collect();

        // Make current state match next state as we're outputting that right now
        self.current_state = self.next_state.clone();
        
        // All the messages
        messages
    }
}
