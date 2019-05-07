
/*
use super::message::Message;

pub struct Grid {
    width: u8,
}

// TODO - undraw & redraw?
pub struct Grid {
    pub fn new(width: u8) -> Self {
        Grid { width, }
    }

    fn led_states(&mut self) -> Vec<(u8, u8, u8)> {
        // Generate ledstates to change current state to next state
        let led_states = (0..self.next.len() as u8)
            .filter(|index| self.next[index as usize] != self.current[index as usize])
            .map(|index| {
                let x = index % self.width;
                let y = index / self.width;
    
                (x, y, self.next[index])
            })
            .collect();

        // Make current state match next state as we're outputting that right now
        self.current = self.next.clone();
        
        // All the led_states
        led_states
    }
}
*/
