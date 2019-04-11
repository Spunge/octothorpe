
use super::{Message, RawMessage};

#[derive(Debug)]
pub struct Scroller {
    buffer: Vec<u8>,
    current_frame: usize,
}

impl Scroller {
    pub fn new(mut string: String) -> Self {
        string.push_str("  ");

        Scroller{
            buffer: Scroller::get_text_as_vector(string),
            current_frame: 0,
        }
    }

    // Increase framecount
    pub fn next_frame(&mut self) {
        self.current_frame += 1;
    }

    pub fn get_frame(&mut self) -> Vec<Message> {
        let mut output = Vec::new();

        for x in 0..8 {
            for y in 0..5 {
                // Get index in our buffer that we should show in frame
                let index = (x + self.current_frame) % (self.buffer.len() / 5) + (self.buffer.len() / 5 * y);

                output.push(Message::new(
                    0,
                    RawMessage::Note([0x90 + x as u8, 0x35 + y as u8, self.buffer[index]]),
                ));
            }
        }

        output
    }

    // Get string as letter vector
    pub fn get_text_as_vector(text: String) -> Vec<u8> {
        // Get letters on each row
        (0..5)
            .map(|row| {
                text.chars()
                    .map(|letter| { 
                        // Get letter vector
                        let vec = Scroller::get_letter(letter);
                        // Get width of letter
                        let width = vec.len() / 5;

                        // Return slice of it based on row
                        let mut sliced = vec[std::ops::Range { start: row * width, end: (row + 1) * width }].to_vec();
                        // Add whitespace
                        sliced.push(0);
                        // Return slice
                        sliced
                    })
                    // Fold into one vector
                    .fold(Vec::new(), |mut acc, mut x| { acc.append(&mut x); acc })
            })
            // Fold into one vector
            .fold(Vec::new(), |mut acc, mut x| { acc.append(&mut x); acc })
    }

    pub fn get_letter(letter: char) -> Vec<u8> {
        match letter {
            '.' => vec![0, 0, 0, 0, 1],
            '\'' => vec![1, 1, 0, 0, 0],
            '!' => vec![1, 1, 1, 0, 1],
            'a' => vec![1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
            'b' => vec![1, 1, 0, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1],
            'c' => vec![1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1],
            'd' => vec![1, 1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1],
            'e' => vec![1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1],
            'f' => vec![1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0],
            'g' => vec![1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1],
            'h' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1],
            'i' => vec![1, 1, 1, 1, 1],
            'j' => vec![1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 1, 0, 1, 1],
            'k' => vec![1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1],
            'l' => vec![1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1],
            'm' => vec![1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1],
            'n' => vec![1, 1, 0, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1],
            'o' => vec![1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1],
            'p' => vec![1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0],
            'q' => vec![1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1],
            'r' => vec![1, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1],
            's' => vec![1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1],
            't' => vec![1, 1, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0],
            'u' => vec![1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1],
            'v' => vec![1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0, 1, 0],
            'w' => vec![1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1, 1, 0],
            'x' => vec![1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1],
            'y' => vec![1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0],
            'z' => vec![1, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 1, 1],
            _ => vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
        }
    }
}
