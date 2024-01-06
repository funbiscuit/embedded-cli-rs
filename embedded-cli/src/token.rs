use crate::utils;

#[derive(Debug, Eq, PartialEq)]
pub struct Tokens<'a> {
    empty: bool,
    tokens: &'a mut str,
}

impl<'a> Tokens<'a> {
    pub fn new(input: &'a mut str) -> Option<Self> {
        // SAFETY: bytes are modified correctly, so they remain utf8
        let bytes = unsafe { input.as_bytes_mut() };
        // 0 is reserved for delimiter
        if bytes.contains(&0) {
            return None;
        }

        let mut insert = 0;
        let mut empty = true;

        if let Some(mut token_start) = bytes.iter().position(|&b| b != b' ') {
            loop {
                if insert > 0 {
                    bytes[insert] = 0;
                    insert += 1;
                }
                let token_end = if bytes[token_start] == b'"' {
                    let mut cursor = token_start + 1;
                    let mut escaped = false;

                    // manually move all bytes since we might need to unescape some of them
                    loop {
                        if cursor >= bytes.len() {
                            break bytes.len();
                        }
                        if escaped {
                            bytes[insert] = bytes[cursor];
                            insert += 1;
                            escaped = false;
                        } else if bytes[cursor] == b'"' {
                            break cursor + 1;
                        } else if bytes[cursor] == b'\\' {
                            escaped = true;
                        } else {
                            bytes[insert] = bytes[cursor];
                            insert += 1;
                        }
                        cursor += 1;
                    }
                } else {
                    // find next space and move everything in bulk
                    let token_end = token_start
                        + bytes[token_start..]
                            .iter()
                            .position(|&b| b == b' ')
                            .unwrap_or(bytes.len() - token_start);
                    bytes.copy_within(token_start..token_end, insert);
                    insert += token_end - token_start;
                    token_end
                };
                empty = false;
                if let Some(start) = bytes[token_end..].iter().position(|&b| b != b' ') {
                    token_start = token_end + start;
                } else {
                    // everything else is whitespace
                    break;
                }
            }
        }

        // SAFETY: bytes are still a valid utf8 sequence
        // insert is inside bytes slice
        let tokens =
            unsafe { core::str::from_utf8_unchecked_mut(bytes.get_unchecked_mut(..insert)) };
        Some(Self { empty, tokens })
    }

    /// Returns raw representation of tokens (delimited with 0)
    pub fn into_raw(self) -> &'a mut str {
        self.tokens
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ str> {
        TokensIter::new(self.tokens, self.empty)
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn remove(&mut self, mut pos: usize) -> Option<&'a mut str> {
        if self.empty {
            return None;
        }

        let mut cursor = 0;
        // SAFETY: bytes are kept valid utf8 during modification
        let bytes = unsafe { self.tokens.as_bytes_mut() };
        while pos > 0 {
            // find byte position for given pos
            if let Some(delim_pos) = bytes[cursor..].iter().position(|&b| b == 0) {
                cursor += delim_pos + 1;
                pos -= 1;
            } else {
                return None;
            }
        }

        if let Some(len) = bytes[cursor..].iter().position(|&b| b == 0) {
            // SAFETY: bytes are kept valid utf8 during modification
            let bytes = unsafe { core::mem::take(&mut self.tokens).as_bytes_mut() };
            // move removed element to the end
            utils::rotate_left(&mut bytes[cursor..], len + 1);
            // SAFETY: bytes are kept valid utf8 during modification
            let bytes = unsafe { core::str::from_utf8_unchecked_mut(bytes) };
            let new_len = bytes.len() - len - 1;
            let (left, right) = bytes.split_at_mut(new_len);
            self.tokens = left;
            // SAFETY: right is len + 1 length long (last byte is 0)
            unsafe { Some(right.get_unchecked_mut(..len)) }
        } else {
            let bytes = core::mem::take(&mut self.tokens);
            if cursor > 0 {
                let (left, right) = bytes.split_at_mut(cursor);
                // SAFETY: left is cursor len, last byte is 0
                self.tokens = unsafe { left.get_unchecked_mut(..cursor - 1) };
                Some(right)
            } else {
                self.empty = true;
                Some(bytes)
            }
        }
    }
}

#[derive(Debug)]
pub struct TokensIter<'a> {
    args: &'a str,
    empty: bool,
}

impl<'a> TokensIter<'a> {
    pub(crate) fn new(args: &'a str, empty: bool) -> Self {
        Self { args, empty }
    }
}

impl<'a> Iterator for TokensIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.empty {
            return None;
        }
        if let Some(pos) = self.args.as_bytes().iter().position(|&b| b == 0) {
            // SAFETY: pos is inside args slice
            let (arg, other) = unsafe {
                (
                    self.args.get_unchecked(..pos),
                    self.args.get_unchecked(pos + 1..),
                )
            };
            self.args = other;
            Some(arg)
        } else {
            self.empty = true;
            Some(self.args)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::token::Tokens;

    #[rstest]
    #[case("", "")]
    #[case("   ", "")]
    #[case("abc", "abc")]
    #[case("  abc ", "abc")]
    #[case("  abc  def ", "abc\0def")]
    #[case("  abc  def gh ", "abc\0def\0gh")]
    #[case("abc  def gh", "abc\0def\0gh")]
    #[case(r#""abc""#, "abc")]
    #[case(r#"  "abc" "#, "abc")]
    #[case(r#"  "  abc " "#, "  abc ")]
    #[case(r#"  "  abc  "#, "  abc  ")]
    #[case(r#"  " abc"   "de fg " "  he  yw""#, " abc\0de fg \0  he  yw")]
    #[case(r#"  "ab \"c\\d\" " "#, r#"ab "c\d" "#)]
    #[case(r#""abc\\""#, r#"abc\"#)]
    fn create(#[case] input: &str, #[case] expected: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let result = Tokens::new(input).unwrap();

        assert_eq!(result.tokens, expected);
        let len = result.tokens.len();
        assert_eq!(&mut input[..len], expected);
    }

    #[rstest]
    #[case("", [None, None, None])]
    #[case(r#""""#, [Some(""), None, None])]
    #[case("abc", [Some("abc"), None, None])]
    #[case("abc def", [Some("abc"), None, None])]
    #[case("abc def gh", [Some("abc"), Some("gh"), None])]
    #[case("abc def gh nmk", [Some("abc"), Some("gh"), None])]
    #[case("abc def gh nmk oprs", [Some("abc"), Some("gh"), Some("oprs")])]
    fn remove(#[case] input: &str, #[case] expected: [Option<&str>; 3]) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let mut result = Tokens::new(input).unwrap();

        for i in 0..3 {
            assert_eq!(
                result.remove(i).map(|s| s.as_bytes()),
                expected[i].map(|s| s.as_bytes())
            );
        }
    }
}
