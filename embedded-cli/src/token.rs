#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tokens<'a> {
    empty: bool,
    tokens: &'a str,
}

impl<'a> Tokens<'a> {
    pub fn new(input: &'a mut str) -> Self {
        // SAFETY: bytes are modified correctly, so they remain utf8
        let bytes = unsafe { input.as_bytes_mut() };

        let mut insert = 0;
        let mut empty = true;

        enum Mode {
            Space,
            Normal,
            Quoted,
            Unescape,
        }

        let mut mode = Mode::Space;

        for cursor_pos in 0..bytes.len() {
            let byte = bytes[cursor_pos];
            match mode {
                Mode::Space => {
                    if byte == b'"' {
                        mode = Mode::Quoted;
                        empty = false;
                        if insert > 0 {
                            bytes[insert] = 0;
                            insert += 1;
                        }
                    } else if byte != b' ' && byte != 0 {
                        mode = Mode::Normal;
                        empty = false;
                        if insert > 0 {
                            bytes[insert] = 0;
                            insert += 1;
                        }
                        bytes[insert] = byte;
                        insert += 1;
                    }
                }
                Mode::Normal => {
                    if byte == b' ' || byte == 0 {
                        mode = Mode::Space;
                    } else {
                        bytes[insert] = byte;
                        insert += 1;
                    }
                }
                Mode::Quoted => {
                    if byte == b'"' || byte == 0 {
                        mode = Mode::Space;
                    } else if byte == b'\\' {
                        mode = Mode::Unescape;
                    } else {
                        bytes[insert] = byte;
                        insert += 1;
                    }
                }
                Mode::Unescape => {
                    bytes[insert] = byte;
                    insert += 1;
                    mode = Mode::Quoted;
                }
            }
        }

        // SAFETY: bytes are still a valid utf8 sequence
        // insert is inside bytes slice
        let tokens = unsafe { core::str::from_utf8_unchecked(bytes.get_unchecked(..insert)) };
        Self { empty, tokens }
    }

    pub fn from_raw(tokens: &'a str, is_empty: bool) -> Self {
        Self {
            empty: is_empty,
            tokens,
        }
    }

    /// Returns raw representation of tokens (delimited with 0)
    pub fn into_raw(self) -> &'a str {
        self.tokens
    }

    pub fn iter(&self) -> TokensIter<'a> {
        TokensIter::new(self.tokens, self.empty)
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

#[derive(Clone, Debug)]
pub struct TokensIter<'a> {
    tokens: &'a str,
    empty: bool,
}

impl<'a> TokensIter<'a> {
    pub fn new(tokens: &'a str, empty: bool) -> Self {
        Self { tokens, empty }
    }

    pub fn into_tokens(self) -> Tokens<'a> {
        Tokens {
            empty: self.empty,
            tokens: self.tokens,
        }
    }
}

impl<'a> Iterator for TokensIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.empty {
            return None;
        }
        if let Some(pos) = self.tokens.as_bytes().iter().position(|&b| b == 0) {
            // SAFETY: pos is inside args slice
            let (arg, other) = unsafe {
                (
                    self.tokens.get_unchecked(..pos),
                    self.tokens.get_unchecked(pos + 1..),
                )
            };
            self.tokens = other;
            Some(arg)
        } else {
            self.empty = true;
            Some(self.tokens)
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
        let result = Tokens::new(input);

        assert_eq!(result.tokens, expected);
        let len = result.tokens.len();
        assert_eq!(&mut input[..len], expected);
    }
}
