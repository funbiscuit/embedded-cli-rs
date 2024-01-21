use crate::{
    autocomplete::{Autocompletion, Request},
    buffer::Buffer,
    utils,
};
use core::{
    fmt::Debug,
    ops::{Bound, RangeBounds},
};

pub struct Editor<B: Buffer> {
    buffer: B,

    /// Where next char will be inserted
    cursor: usize,

    /// How many bytes of valid utf-8 are stored in buffer
    valid: usize,
}

impl<B: Buffer> Debug for Editor<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Editor")
            .field("buffer", &self.buffer.as_slice())
            .field("cursor", &self.cursor)
            .field("valid", &self.valid)
            .finish()
    }
}

impl<B: Buffer> Editor<B> {
    pub fn new(buffer: B) -> Self {
        Self {
            buffer,
            cursor: 0,
            valid: 0,
        }
    }

    /// Calls given function to create autocompletion of current input
    pub fn autocompletion(&mut self, f: impl FnOnce(Request<'_>, &mut Autocompletion<'_>)) {
        // SAFETY: self.valid is always less than or equal to buffer len
        let (text, buf) = unsafe { utils::split_at_mut(self.buffer.as_slice_mut(), self.valid) };

        // SAFETY: buffer stores only valid utf-8 bytes 0..valid range
        let mut text = unsafe { core::str::from_utf8_unchecked(text) };
        let mut removed_spaces = 0;

        if let Some(pos) = utils::char_byte_index(text, self.cursor) {
            // cursor is inside text, so trim all whitespace, that is on the right to the cursor
            let right = &text.as_bytes()[pos..];
            let pos2 = right
                .iter()
                .rev()
                .position(|&b| b != b' ')
                .unwrap_or(right.len());
            // SAFETY: pos2 is at the char boundary
            text = unsafe { text.get_unchecked(..text.len() - pos2) };
            removed_spaces = pos2;
        }

        if let Some(request) = Request::from_input(text) {
            let mut autocompletion = Autocompletion::new(buf);
            f(request, &mut autocompletion);

            // process autocompletion
            if let Some(autocompleted) = autocompletion.autocompleted() {
                let mut bytes = autocompleted.len();
                let is_partial = autocompletion.is_partial();
                if !is_partial && buf.len() > bytes {
                    buf[bytes] = b' ';
                    bytes += 1;
                }
                if removed_spaces > 0 {
                    // shift autocompleted text to the left
                    self.buffer
                        .as_slice_mut()
                        .copy_within(self.valid.., self.valid - removed_spaces);
                    self.valid -= removed_spaces;
                }
                self.valid += bytes;
                self.cursor = self.len();
            }
        }
    }

    pub fn clear(&mut self) {
        self.valid = 0;
        self.cursor = 0;
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn insert(&mut self, text: &str) -> Option<&str> {
        let remaining = self.buffer.len() - self.valid;
        let chars = text.chars().count();
        let text = text.as_bytes();
        if remaining < text.len() {
            //TODO: try to grow buffer
            return None;
        }
        let cursor = if let Some(cursor) = utils::char_byte_index(self.text(), self.cursor) {
            self.buffer
                .as_slice_mut()
                .copy_within(cursor..self.valid, cursor + text.len());
            cursor
        } else {
            self.valid
        };
        self.buffer.as_slice_mut()[cursor..cursor + text.len()].copy_from_slice(text);
        let text = &self.buffer.as_slice()[cursor..cursor + text.len()];
        self.cursor += chars;
        self.valid += text.len();
        //SAFETY: we just copied valid utf-8 from &str to this location
        Some(unsafe { core::str::from_utf8_unchecked(text) })
    }

    pub fn len(&self) -> usize {
        //TODO: use another usize to store len
        self.text().chars().count()
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else {
            false
        }
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor < self.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    /// Removes char at cursor position
    pub fn remove(&mut self) {
        let cursor_pos = utils::char_byte_index(self.text(), self.cursor);
        let next_pos = if let Some(cursor_pos) = cursor_pos {
            // SAFETY: cursor_pos is at char boundary
            let text = unsafe { self.text().get_unchecked(cursor_pos..) };
            utils::char_byte_index(text, 1).map(|s| s + cursor_pos)
        } else {
            None
        };

        match (cursor_pos, next_pos) {
            (Some(cursor), None) => {
                // we are at the last char, so just decrease valid size
                self.valid = cursor;
            }
            (Some(cursor), Some(next)) => {
                self.buffer
                    .as_slice_mut()
                    .copy_within(next..self.valid, cursor);
                self.valid -= next - cursor;
            }
            _ => {} // nothing to remove
        }
    }

    pub fn text(&self) -> &str {
        // SAFETY: buffer stores only valid utf-8 bytes 0..valid range
        unsafe { core::str::from_utf8_unchecked(&self.buffer.as_slice()[..self.valid]) }
    }

    pub fn text_mut(&mut self) -> &mut str {
        // SAFETY: buffer stores only valid utf-8 bytes 0..valid range
        unsafe { core::str::from_utf8_unchecked_mut(&mut self.buffer.as_slice_mut()[..self.valid]) }
    }

    /// Returns text in subrange of this editor. start is including, end is exclusive
    pub fn text_range(&self, range: impl RangeBounds<usize>) -> &str {
        let (start, num_chars) = match (range.start_bound(), range.end_bound()) {
            (Bound::Included(start), Bound::Included(end)) => {
                if end < start {
                    return "";
                }
                (*start, Some(end - start + 1))
            }
            (Bound::Included(start), Bound::Excluded(end)) => {
                if end <= start {
                    return "";
                }
                (*start, Some(end - start))
            }
            (Bound::Unbounded, Bound::Included(end)) => (0, Some(end + 1)),
            (Bound::Unbounded, Bound::Excluded(end)) => {
                if *end == 0 {
                    return "";
                }
                (0, Some(*end))
            }
            (Bound::Included(start), Bound::Unbounded) => (*start, None),
            (Bound::Unbounded, Bound::Unbounded) => (0, None),
            (Bound::Excluded(_), _) => unreachable!(),
        };

        let text = self.text();

        let (start, end) = if let Some(num_chars) = num_chars {
            if let Some(pos) = utils::char_byte_index(text, start) {
                // SAFETY: pos is at char boundary
                let text = unsafe { text.get_unchecked(pos..) };
                let b = utils::char_byte_index(text, num_chars).map(|s| s + pos);
                (Some(pos), b)
            } else {
                (None, None)
            }
        } else {
            (utils::char_byte_index(text, start), None)
        };

        match (start, end) {
            (Some(start), Some(end)) => {
                // SAFETY: we take substring from valid utf8 slice
                unsafe { core::str::from_utf8_unchecked(&text.as_bytes()[start..end]) }
            }
            (Some(start), None) => {
                // SAFETY: we take substring from valid utf8 slice
                unsafe { core::str::from_utf8_unchecked(&text.as_bytes()[start..]) }
            }
            _ => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use core::ops::RangeBounds;
    use std::string::String;

    use rstest::rstest;

    use super::Editor;

    #[test]
    fn add_chars_to_back() {
        let mut editor = Editor::new([0; 128]);

        let text = "abcdабвг佐佗佟𑿁𑿆𑿌";

        for (i, b) in text.chars().enumerate() {
            let mut buffer = [0u8; 4];
            editor.insert(b.encode_utf8(&mut buffer));
            let exp: String = text.chars().take(i + 1).collect();
            assert_eq!(editor.text(), &exp);
        }
    }

    #[test]
    fn add_chars_to_front() {
        let mut editor = Editor::new([0; 128]);

        let text = "abcdабвг佐佗佟𑿁𑿆𑿌";

        for (i, b) in text.chars().enumerate() {
            let mut buffer = [0u8; 4];
            editor.insert(b.encode_utf8(&mut buffer));
            assert!(editor.move_left());
            let exp = text
                .chars()
                .take(i + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            assert_eq!(editor.text(), &exp);
        }
    }

    #[rstest]
    #[case("abc", 1, "Ж", "abЖc")]
    #[case("abc", 2, "Ж", "aЖbc")]
    #[case("abc", 3, "Ж ", "Ж abc")]
    #[case("abc", 4, "Ж ", "Ж abc")]
    #[case("adbc佐佗𑿌", 2, "Ж", "adbc佐Ж佗𑿌")]
    fn move_left_insert(
        #[case] initial: &str,
        #[case] count: usize,
        #[case] inserted: &str,
        #[case] expected: &str,
    ) {
        let mut editor = Editor::new([0; 128]);

        editor.insert(initial);

        for _ in 0..count {
            editor.move_left();
        }

        editor.insert(inserted);

        assert_eq!(editor.text_range(..), expected);
    }

    #[rstest]
    #[case("abc", 3, 1, "Ж", "aЖbc")]
    #[case("абв", 3, 2, "Ж", "абЖв")]
    #[case("абв", 1, 1, "Ж ", "абвЖ ")]
    #[case("абв", 1, 2, "Ж ", "абвЖ ")]
    #[case("adbc佐佗𑿌", 4, 2, "Ж", "adbc佐Ж佗𑿌")]
    fn move_left_then_right_insert(
        #[case] initial: &str,
        #[case] count_left: usize,
        #[case] count_right: usize,
        #[case] inserted: &str,
        #[case] expected: &str,
    ) {
        let mut editor = Editor::new([0; 128]);

        editor.insert(initial);

        for _ in 0..count_left {
            editor.move_left();
        }
        for _ in 0..count_right {
            editor.move_right();
        }

        editor.insert(inserted);

        assert_eq!(editor.text_range(..), expected);
    }

    #[test]
    fn remove() {
        let mut editor = Editor::new([0; 128]);

        editor.insert("adbc佐佗𑿌");
        assert_eq!(editor.cursor, 7);
        editor.remove();

        assert_eq!(editor.text(), "adbc佐佗𑿌");
        assert_eq!(editor.cursor, 7);

        editor.move_left();
        editor.remove();

        assert_eq!(editor.text(), "adbc佐佗");
        assert_eq!(editor.cursor, 6);

        editor.move_left();
        editor.move_left();
        editor.remove();

        assert_eq!(editor.text(), "adbc佗");
        assert_eq!(editor.cursor, 4);

        editor.move_left();
        editor.move_left();
        editor.move_left();
        editor.remove();

        assert_eq!(editor.text(), "abc佗");
        assert_eq!(editor.cursor, 1);

        editor.move_left();
        editor.remove();

        assert_eq!(editor.text(), "bc佗");
        assert_eq!(editor.cursor, 0);

        editor.remove();
        assert_eq!(editor.text(), "c佗");

        editor.remove();
        assert_eq!(editor.text(), "佗");

        editor.remove();
        assert_eq!(editor.text(), "");
    }

    #[rstest]
    #[case(1, "adbc佐佗")]
    #[case(2, "adbc佐𑿌")]
    #[case(3, "adbc佗𑿌")]
    #[case(4, "adb佐佗𑿌")]
    #[case(5, "adc佐佗𑿌")]
    #[case(6, "abc佐佗𑿌")]
    #[case(7, "dbc佐佗𑿌")]
    fn remove_inside(#[case] dist: usize, #[case] expected: &str) {
        let mut editor = Editor::new([0; 128]);

        editor.insert("adbc佐佗𑿌");

        for _ in 0..dist {
            editor.move_left();
        }
        editor.remove();

        assert_eq!(editor.text(), expected);
    }

    #[rstest]
    #[case(.., "adbc佐佗𑿌")]
    #[case(..2, "ad")]
    #[case(0..2, "ad")]
    #[case(2.., "bc佐佗𑿌")]
    #[case(5.., "佗𑿌")]
    #[case(..6, "adbc佐佗")]
    #[case(..7, "adbc佐佗𑿌")]
    #[case(..=6, "adbc佐佗𑿌")]
    #[case(3..5, "c佐")]
    #[case(3..6, "c佐佗")]
    #[case(3..3, "")]
    #[case(..0, "")]
    #[case(1..=0, "")]
    #[case(5..=5, "佗")]
    fn text_range(#[case] range: impl RangeBounds<usize>, #[case] expected: &str) {
        let mut editor = Editor::new([0; 128]);

        editor.insert("adbc佐佗𑿌");

        assert_eq!(editor.text_range(range), expected);
    }
}
