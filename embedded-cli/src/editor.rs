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
        if self.cursor < self.len() {
            //autocompletion is possible only when cursor is at the end
            return;
        }

        // SAFETY: self.valid is always less than or equal to buffer len
        let (text, buf) = unsafe { utils::split_at_mut(self.buffer.as_slice_mut(), self.valid) };

        // SAFETY: buffer stores only valid utf-8 bytes 0..valid range
        let text = unsafe { core::str::from_utf8_unchecked(text) };

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

        let cursor = if let Some(cursor) = self
            .text()
            .char_indices()
            .skip(self.cursor)
            .map(|(pos, _)| pos)
            .next()
        {
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

    /// Removes char at cursor position
    pub fn remove(&mut self) {
        let mut it = self
            .text()
            .char_indices()
            .skip(self.cursor)
            .map(|(pos, _)| pos);

        let cursor_pos = it.next();
        let next_pos = it.next();

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
        let mut it = text.char_indices().map(|(i, _)| i).skip(start);

        let (start, end) = if let Some(num_chars) = num_chars {
            // num chars always > 0
            (it.next(), it.nth(num_chars - 1))
        } else {
            (it.next(), None)
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
