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
use regex::Regex;

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

    pub fn receive_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.receive_byte(byte)
        }
    }

    /// Returns vector of terminal lines
    /// and current cursor position (cursor column)
    ///
    /// end of lines is trimmed so input "ab " is displayed as "ab" (not "ab ")
    pub fn view(&self) -> (Vec<String>, usize) {
        let mut output = vec!["".to_string()];

        // cursor is char position (not utf8 byte position)
        let mut cursor = 0;

        let mut received = std::str::from_utf8(&self.received)
            .expect("Received bytes must form utf8 string")
            .to_string();

        // Simple regex for CSI sequences
        let seq_re = Regex::new(r"\x1b\[([\x30-\x3f]*[\x20-\x2f]*[\x40-\x7e])").unwrap();

        while !received.is_empty() {
            let (normal, seq) = if let Some(seq_match) = seq_re.find(&received) {
                let seq = seq_match.as_str().to_string();
                if seq_match.start() > 0 {
                    let normal = received[..seq_match.start()].to_string();
                    received = received[seq_match.end()..].to_string();
                    (Some(normal), Some(seq))
                } else {
                    received = received[seq_match.end()..].to_string();
                    (None, Some(seq))
                }
            } else {
                let normal = received;
                received = "".to_string();
                (Some(normal), None)
            };

            if let Some(normal) = normal {
                for c in normal.chars().into_iter() {
                    match c {
                        '\r' => {
                            cursor = 0;
                        }
                        '\n' => {
                            // start new line (but keep cursor position)
                            output.push("".to_string());
                        }
                        c if c >= ' ' => {
                            let current = output.last_mut().unwrap();
                            if current.chars().count() > cursor {
                                current
                                    .remove(current.char_indices().skip(cursor).next().unwrap().0);
                            } else {
                                while current.chars().count() < cursor {
                                    current.push(' ');
                                }
                            }
                            current.insert(cursor, c);
                            cursor += 1;
                        }
                        _ => unimplemented!(),
                    }
                }
            }

            if let Some(seq) = seq {
                let current = output.last_mut().unwrap();
                match seq.as_str() {
                    // cursor forward
                    "\x1B[C" => {
                        cursor += 1;
                    }
                    // cursor backward
                    "\x1B[D" => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                    }
                    // delete char
                    "\x1B[P" => {
                        if current.chars().count() > cursor {
                            current.remove(current.char_indices().skip(cursor).next().unwrap().0);
                        }
                    }
                    // insert char
                    "\x1B[@" => {
                        if current.chars().count() > cursor {
                            current
                                .insert(current.char_indices().skip(cursor).next().unwrap().0, ' ');
                        }
                    }
                    // clear whole line
                    "\x1B[2K" => {
                        // cursor position does not change
                        current.clear();
                    }
                    _ => unimplemented!(),
                }
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

        // line feed doesn't reset cursor position
        assert_terminal!(&terminal, 2, vec!["ab", ""]);

        terminal.receive_byte(b'c');
        assert_terminal!(&terminal, 3, vec!["ab", "  c"]);
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
    fn move_forward_backward() {
        let mut terminal = Terminal::new();

        terminal.receive_bytes(b"abc");
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        assert_terminal!(&terminal, 2, vec!["abc"]);

        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        assert_terminal!(&terminal, 1, vec!["abc"]);

        terminal.receive_byte(b'd');
        assert_terminal!(&terminal, 2, vec!["adc"]);

        terminal.receive_byte(b'e');
        terminal.receive_byte(b'f');

        assert_terminal!(&terminal, 4, vec!["adef"]);

        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::CURSOR_BACKWARD);

        assert_terminal!(&terminal, 1, vec!["adef"]);

        terminal.receive_bytes(codes::CURSOR_FORWARD);

        assert_terminal!(&terminal, 2, vec!["adef"]);

        terminal.receive_byte(b'b');

        assert_terminal!(&terminal, 3, vec!["adbf"]);
    }

    #[test]
    fn delete_chars() {
        let mut terminal = Terminal::new();

        terminal.receive_bytes(b"abc");
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::DELETE_CHAR);
        assert_terminal!(&terminal, 2, vec!["ab"]);

        terminal.receive_bytes(b"def");
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        assert_terminal!(&terminal, 2, vec!["abdef"]);

        terminal.receive_bytes(codes::DELETE_CHAR);
        assert_terminal!(&terminal, 2, vec!["abef"]);

        terminal.receive_bytes(codes::DELETE_CHAR);
        assert_terminal!(&terminal, 2, vec!["abf"]);

        terminal.receive_byte(b'e');
        assert_terminal!(&terminal, 3, vec!["abe"]);
    }

    #[test]
    fn insert_chars() {
        let mut terminal = Terminal::new();

        terminal.receive_bytes(b"abc");
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::INSERT_CHAR);
        assert_terminal!(&terminal, 2, vec!["ab c"]);

        terminal.receive_byte(b'd');
        assert_terminal!(&terminal, 3, vec!["abdc"]);

        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::CURSOR_BACKWARD);
        terminal.receive_bytes(codes::INSERT_CHAR);
        assert_terminal!(&terminal, 1, vec!["a bdc"]);

        terminal.receive_bytes(codes::INSERT_CHAR);
        assert_terminal!(&terminal, 1, vec!["a  bdc"]);
    }

    #[test]
    fn clear_line() {
        let mut terminal = Terminal::new();

        terminal.receive_bytes(b"abc");
        terminal.receive_bytes(codes::CLEAR_LINE);
        assert_terminal!(&terminal, 3, vec![""]);

        terminal.receive_byte(b'd');
        assert_terminal!(&terminal, 4, vec!["   d"]);
    }
}
