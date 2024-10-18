use crate::utils;

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Request<'a> {
    /// Request to autocomplete given text to command name
    CommandName(&'a str),
}

impl<'a> Request<'a> {
    pub fn from_input(input: &'a str) -> Option<Self> {
        let input = utils::trim_start(input);

        if input.is_empty() {
            return None;
        }

        // if no space given, then only command name is entered so we complete it
        if !input.contains(' ') {
            Some(Request::CommandName(input))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Autocompletion<'a> {
    autocompleted: Option<usize>,
    buffer: &'a mut [u8],
    partial: bool,
}

impl<'a> Autocompletion<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            autocompleted: None,
            buffer,
            partial: false,
        }
    }

    pub fn autocompleted(&self) -> Option<&str> {
        self.autocompleted.map(|len| {
            // SAFETY: we store only &str in this buffer, so it is a valid utf-8 sequence
            unsafe { core::str::from_utf8_unchecked(&self.buffer[..len]) }
        })
    }

    /// Whether autocompletion is partial
    /// and further input is required
    pub fn is_partial(&self) -> bool {
        self.partial
    }

    /// Mark this autocompletion as partial
    pub fn mark_partial(&mut self) {
        self.partial = true;
    }

    /// Merge this autocompletion with another one
    pub fn merge_autocompletion(&mut self, autocompletion: &str) {
        if autocompletion.is_empty() || self.buffer.is_empty() {
            self.partial = self.partial
                || self.autocompleted.is_some()
                || (self.buffer.is_empty() && !autocompletion.is_empty());
            self.autocompleted = Some(0);
            return;
        }

        // compare new autocompletion to existing and keep
        // only common prefix
        let len = match self.autocompleted() {
            Some(current) => utils::common_prefix_len(autocompletion, current),
            None => autocompletion.len(),
        };

        if len > self.buffer.len() {
            // if buffer is full with this autocompletion, there is not much sense in doing it
            // since user will not be able to type anything else
            // so just do nothing with it
        } else {
            self.partial =
                self.partial || len < autocompletion.len() || self.autocompleted.is_some();
            // SAFETY: we checked that len is no longer than buffer len (and is at most autocompleted len)
            // and these two buffers do not overlap since mutable reference to buffer is exclusive
            unsafe {
                utils::copy_nonoverlapping(autocompletion.as_bytes(), self.buffer, len);
            }
            self.autocompleted = Some(len);
        };
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::autocomplete::Autocompletion;

    #[test]
    fn no_merge() {
        let mut input = [0; 64];

        let autocompletion = Autocompletion::new(&mut input);

        assert!(!autocompletion.is_partial());
        assert_eq!(autocompletion.autocompleted(), None);
    }

    #[rstest]
    #[case("abc", "abc")]
    #[case("", "")]
    fn merge_single(#[case] text: &str, #[case] expected: &str) {
        let mut input = [0; 64];

        let mut autocompletion = Autocompletion::new(&mut input);

        autocompletion.merge_autocompletion(text);

        assert!(!autocompletion.is_partial());
        assert_eq!(autocompletion.autocompleted(), Some(expected));
        assert_eq!(&input[..expected.len()], expected.as_bytes());
    }

    #[rstest]
    #[case("abc1", "abc2", "abc")]
    #[case("ab", "abc", "ab")]
    #[case("abc", "ab", "ab")]
    #[case("", "ab", "")]
    #[case("ab", "", "")]
    #[case("abc", "def", "")]
    fn merge_multiple(#[case] text1: &str, #[case] text2: &str, #[case] expected: &str) {
        let mut input = [0; 64];

        let mut autocompletion = Autocompletion::new(&mut input);

        autocompletion.merge_autocompletion(text1);
        autocompletion.merge_autocompletion(text2);

        assert!(autocompletion.is_partial());
        assert_eq!(autocompletion.autocompleted(), Some(expected));
        assert_eq!(&input[..expected.len()], expected.as_bytes());
    }
}
