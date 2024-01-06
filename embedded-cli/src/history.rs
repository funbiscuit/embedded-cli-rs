use crate::buffer::Buffer;

#[derive(Debug)]
pub struct History<B: Buffer> {
    /// Buffer that stores element bytes.
    /// Elements are stored null separated, thus no null
    /// bytes are allowed in elements themselves
    /// Newer elements are placed to the right of previous element
    /// Last element is not null terminated
    buffer: B,

    /// Index of first byte of currently selected element
    cursor: Option<usize>,

    /// How many bytes of buffer are already used by elements
    used: usize,
}

impl<B: Buffer> History<B> {
    pub fn new(buffer: B) -> Self {
        Self {
            buffer,
            cursor: None,
            used: 0,
        }
    }

    /// Return next element from history, that is newer, than currently selected.
    /// Return None if there is no newer elements
    pub fn next_newer(&mut self) -> Option<&str> {
        match self.cursor {
            Some(cursor) => {
                let new_cursor = self.buffer.as_slice()[cursor..self.used - 1]
                    .iter()
                    .position(|b| b == &0)
                    .map(|pos| cursor + pos + 1);
                if let Some(new_cursor) = new_cursor {
                    let element_end = new_cursor
                        + self.buffer.as_slice()[new_cursor..]
                            .iter()
                            .position(|b| b == &0)
                            .expect("all elements are null terminated");

                    let element = unsafe {
                        core::str::from_utf8_unchecked(
                            &self.buffer.as_slice()[new_cursor..element_end],
                        )
                    };
                    self.cursor = Some(new_cursor);
                    Some(element)
                } else {
                    self.cursor = None;
                    None
                }
            }
            _ => None,
        }
    }

    /// Return next element from history, that is older, than currently selected.
    /// Return None if there is no older elements
    pub fn next_older(&mut self) -> Option<&str> {
        let cursor = match self.cursor {
            Some(cursor) if cursor > 0 => cursor,
            None if self.used > 0 => self.used,
            _ => return None,
        };

        let new_cursor = self.buffer.as_slice()[..cursor - 1]
            .iter()
            .rev()
            .position(|b| b == &0)
            .map(|pos| cursor - 1 - pos)
            .unwrap_or(0);
        let element = unsafe {
            core::str::from_utf8_unchecked(&self.buffer.as_slice()[new_cursor..cursor - 1])
        };
        self.cursor = Some(new_cursor);
        Some(element)
    }

    /// Push given text to history. Text must not contain any null bytes. Otherwise
    /// text is not pushed to history and just ignored.
    pub fn push(&mut self, text: &str) {
        // extra byte is added to text len since we need to null terminate it
        if text.as_bytes().contains(&0) || text.len() + 1 > self.buffer.len() || text.is_empty() {
            return;
        }

        self.cursor = None;

        // check if duplicate is given, then we should remove it first
        // this is a bit slower than manually comparing all bytes, but easier to write
        match self.next_older() {
            Some(existing) if existing == text => {
                // element already is added and is newest among others
                // so we have nothing to do
                self.cursor = None;
                return;
            }
            _ => {}
        }

        while let Some(existing) = self.next_older() {
            if existing == text {
                let removing_start = self.cursor.unwrap();
                let removing_end = removing_start + text.len() + 1;

                self.buffer
                    .as_slice_mut()
                    .copy_within(removing_end..self.used, removing_start);
                self.used -= text.len() + 1;
                break;
            }
        }
        self.cursor = None;

        // remove old commands to free space if its not enough
        if self.buffer.len() < self.used + text.len() + 1 {
            // self.used is at least 2 bytes (1 for element and 1 for null terminator)
            // how many bytes we should free, this is at least 1 byte
            let required = self.used + text.len() + 1 - self.buffer.len();
            if required >= self.used {
                self.used = 0;
            } else {
                // how many bytes we are removing, so whole command is removed
                let removing = required
                    + self.buffer.as_slice()[required - 1..self.used]
                        .iter()
                        .position(|b| b == &0)
                        .expect("Last used byte is always 0");

                if removing < self.used {
                    self.buffer
                        .as_slice_mut()
                        .copy_within(removing..self.used, 0);
                    self.used -= removing;
                } else {
                    self.used = 0;
                }
            }
        }

        // now we have enough space after self.used to insert element
        let null_pos = self.used + text.len();
        self.buffer.as_slice_mut()[self.used..null_pos].copy_from_slice(text.as_bytes());
        self.buffer.as_slice_mut()[null_pos] = 0;
        self.used += text.len() + 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::history::History;

    #[test]
    fn empty() {
        let mut history = History::new([0; 64]);

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), None);
    }

    #[test]
    fn text_with_nulls() {
        let mut history = History::new([0; 64]);

        history.push("ab\0c");

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), None);
    }

    #[test]
    fn navigation() {
        let mut history = History::new([0; 32]);

        history.push("abc");
        history.push("def");
        history.push("ghi");

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), Some("ghi"));
        assert_eq!(history.next_older(), Some("def"));
        assert_eq!(history.next_older(), Some("abc"));
        assert_eq!(history.next_older(), None);
        assert_eq!(history.next_newer(), Some("def"));
        assert_eq!(history.next_newer(), Some("ghi"));
        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), Some("ghi"));
        assert_eq!(history.next_older(), Some("def"));

        history.push("jkl");

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), Some("jkl"));
        assert_eq!(history.next_older(), Some("ghi"));
        assert_eq!(history.next_older(), Some("def"));

        history.push("ghi");

        assert_eq!(history.next_older(), Some("ghi"));
        assert_eq!(history.next_older(), Some("jkl"));
        assert_eq!(history.next_older(), Some("def"));
        assert_eq!(history.next_older(), Some("abc"));
        assert_eq!(history.next_older(), None);
    }

    #[test]
    fn overflow_small() {
        let mut history = History::new([0; 12]);

        history.push("abc");
        history.push("def");
        history.push("ghi");
        history.push("jkl");

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), Some("jkl"));
        assert_eq!(history.next_older(), Some("ghi"));
        assert_eq!(history.next_older(), Some("def"));
        assert_eq!(history.next_older(), None);
    }

    #[test]
    fn overflow_big() {
        let mut history = History::new([0; 10]);

        history.push("abc");
        history.push("def");
        history.push("ghijklm");

        assert_eq!(history.next_newer(), None);
        assert_eq!(history.next_older(), Some("ghijklm"));
        assert_eq!(history.next_older(), None);
    }

    #[test]
    fn duplicate_when_full() {
        let mut history = History::new([0; 10]);

        history.push("abc");
        history.push("defgh");

        assert_eq!(history.next_older(), Some("defgh"));
        assert_eq!(history.next_older(), Some("abc"));
        assert_eq!(history.next_older(), None);

        history.push("abc");

        assert_eq!(history.next_older(), Some("abc"));
        assert_eq!(history.next_older(), Some("defgh"));
        assert_eq!(history.next_older(), None);

        history.push("abc");

        assert_eq!(history.next_older(), Some("abc"));
        assert_eq!(history.next_older(), Some("defgh"));
        assert_eq!(history.next_older(), None);
    }
}
