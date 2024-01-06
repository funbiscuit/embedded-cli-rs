macro_rules! assert_terminal {
    ($terminal:expr, $curs:expr, $b:expr) => {
        let expected = $b;
        let terminal = $terminal;
        let (lines, cursor) = terminal.view();

        assert_eq!(lines, expected);
        assert_eq!(cursor, $curs);
    };
}

pub(crate) use assert_terminal;
use embedded_cli::codes;

#[derive(Debug)]
pub struct Terminal {
    /// All received bytes
    received: Vec<u8>,
}

impl Terminal {
    pub fn new() -> Self {
        Self { received: vec![] }
    }

    pub fn receive_byte(&mut self, byte: u8) {
        self.received.push(byte);
    }

    /// Returns vector of terminal lines
    /// and current cursor position (cursor column)
    ///
    /// end of lines is trimmed so input "ab\b " is displayed as "ab" (not "ab ")
    pub fn view(&self) -> (Vec<String>, usize) {
        let mut output = vec!["".to_string()];

        let mut cursor = 0;

        for c in self.received.iter().copied() {
            if c == codes::BACKSPACE {
                if cursor > 0 {
                    // backspace only moves cursor, without clearing text
                    cursor -= 1;
                }
            } else if c == codes::CARRIAGE_RETURN {
                cursor = 0;
            } else if c == codes::LINE_FEED {
                // reset cursor and start new line
                cursor = 0;
                output.push("".to_string());
            } else if let Some(c) = char::from_u32(c as u32) {
                let line = output.last_mut().unwrap();

                if line.len() > cursor {
                    line.remove(cursor);
                }
                line.insert(cursor, c);
                cursor += 1;
            }
        }

        let output = output
            .into_iter()
            .map(|l| l.trim_end().to_string())
            .collect();

        (output, cursor)
    }
}

#[cfg(test)]
mod tests {
    use embedded_cli::codes;

    use super::Terminal;

    #[test]
    fn simple() {
        let mut terminal = Terminal::new();

        assert_terminal!(&terminal, 0, vec![""]);

        terminal.receive_byte(b'a');
        terminal.receive_byte(b'b');
        terminal.receive_byte(b'c');

        assert_terminal!(terminal, 3, vec!["abc"]);
    }

    #[test]
    fn line_feeds() {
        let mut terminal = Terminal::new();

        terminal.receive_byte(b'a');
        terminal.receive_byte(b'b');
        terminal.receive_byte(codes::LINE_FEED);
        terminal.receive_byte(b'c');

        assert_terminal!(terminal, 1, vec!["ab", "c"]);
    }

    #[test]
    fn carriage_return() {
        let mut terminal = Terminal::new();

        terminal.receive_byte(b'a');
        terminal.receive_byte(b'b');
        terminal.receive_byte(codes::CARRIAGE_RETURN);

        assert_terminal!(&terminal, 0, vec!["ab"]);

        terminal.receive_byte(b'c');

        assert_terminal!(terminal, 1, vec!["cb"]);
    }

    #[test]
    fn back_space() {
        let mut terminal = Terminal::new();

        terminal.receive_byte(b'a');
        terminal.receive_byte(b'b');
        terminal.receive_byte(b'c');
        terminal.receive_byte(codes::BACKSPACE);
        assert_terminal!(&terminal, 2, vec!["abc"]);

        terminal.receive_byte(codes::BACKSPACE);
        assert_terminal!(&terminal, 1, vec!["abc"]);

        terminal.receive_byte(b'd');
        assert_terminal!(&terminal, 2, vec!["adc"]);

        terminal.receive_byte(b'e');
        terminal.receive_byte(b'f');

        assert_terminal!(terminal, 4, vec!["adef"]);
    }
}
