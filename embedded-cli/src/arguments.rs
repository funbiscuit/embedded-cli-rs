use crate::token::{Tokens, TokensIter};

#[derive(Debug, Eq, PartialEq)]
pub enum Arg<'a> {
    /// Used to represent `--`.
    /// After double dash all other args
    /// will always be `ArgToken::Value`
    DoubleDash,

    /// Long option. Only name is stored (without `--`)
    ///
    /// In `get --config normal -f file -vs`
    /// `--config` will be a long option with name `config`
    LongOption(&'a str),

    /// Short option. Only single ASCII char is stored (without `-`).
    /// UTF-8 here is not supported.
    ///
    /// In `get --config normal -f file -vs`
    /// `-f` and `-vs` will be short options.
    /// `v` and `s` are treated as written separately (as '-v -s`)
    ShortOption(char),

    /// Value of an option or an argument.
    ///
    /// In `get --config normal -v file`
    /// `normal` and `file` will be a value
    Value(&'a str),
}

#[derive(Clone, Debug, Eq)]
pub struct ArgList<'a> {
    args: &'a str,
    empty: bool,
}

impl<'a> ArgList<'a> {
    /// Create new arg list from given tokens
    pub fn new(tokens: Tokens<'a>) -> Self {
        let empty = tokens.is_empty();
        Self {
            args: tokens.into_raw(),
            empty,
        }
    }

    pub fn args(&self) -> impl Iterator<Item = Result<Arg<'a>, ArgError>> {
        ArgsIter::new(TokensIter::new(self.args, self.empty))
    }

    pub fn iter(&self) -> impl Iterator<Item = &'a str> {
        TokensIter::new(self.args, self.empty)
    }
}

impl<'a> PartialEq for ArgList<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ArgError {
    NonAsciiShortOption,
}

struct ArgsIter<'a> {
    values_only: bool,

    /// Short options (ASCII chars) that
    /// are left from previous iteration
    leftover: &'a [u8],

    tokens: TokensIter<'a>,
}

impl<'a> ArgsIter<'a> {
    fn new(tokens: TokensIter<'a>) -> Self {
        Self {
            values_only: false,
            leftover: &[],
            tokens,
        }
    }
}

impl<'a> Iterator for ArgsIter<'a> {
    type Item = Result<Arg<'a>, ArgError>;

    fn next(&mut self) -> Option<Self::Item> {
        fn process_leftover<'a>(byte: u8) -> Result<Arg<'a>, ArgError> {
            if byte.is_ascii_alphabetic() {
                // SAFETY: we checked that this is alphabetic ASCII
                Ok(Arg::ShortOption(unsafe {
                    char::from_u32_unchecked(byte as u32)
                }))
            } else {
                Err(ArgError::NonAsciiShortOption)
            }
        }

        if !self.leftover.is_empty() {
            let byte = self.leftover[0];
            self.leftover = &self.leftover[1..];
            return Some(process_leftover(byte));
        }

        let raw = self.tokens.next()?;
        let bytes = raw.as_bytes();

        if self.values_only {
            return Some(Ok(Arg::Value(raw)));
        }

        let token = if bytes.len() > 1 && bytes[0] == b'-' {
            if bytes[1] == b'-' {
                if bytes.len() == 2 {
                    self.values_only = true;
                    Arg::DoubleDash
                } else {
                    Arg::LongOption(unsafe { raw.get_unchecked(2..) })
                }
            } else {
                self.leftover = &bytes[2..];
                return Some(process_leftover(bytes[1]));
            }
        } else {
            Arg::Value(raw)
        };

        Some(Ok(token))
    }
}

pub trait FromArgument<'a> {
    fn from_arg(arg: &'a str) -> Result<Self, &'static str>
    where
        Self: Sized;
}

impl<'a> FromArgument<'a> for &'a str {
    fn from_arg(arg: &'a str) -> Result<Self, &'static str> {
        Ok(arg)
    }
}

macro_rules! impl_arg_fromstr {
    ($id:ident) => (
        impl<'a> FromArgument<'a> for $id {
            fn from_arg(arg: &'a str) -> Result<Self, &'static str> {
                arg.parse().map_err(|_| "invalid value")
            }
        }
    );

    ($id:ident, $($ids:ident),+) => (
        impl_arg_fromstr!{$id}
        impl_arg_fromstr!{$($ids),+}
    )
}

impl_arg_fromstr! {char, bool, u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, usize, isize, f32, f64}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{arguments::ArgList, token::Tokens};

    use super::{Arg, ArgError};

    #[rstest]
    #[case("arg1 --option1 val1 -f val2 -vs", &[
        Ok(Arg::Value("arg1")), 
        Ok(Arg::LongOption("option1")),
        Ok(Arg::Value("val1")),
        Ok(Arg::ShortOption('f')),
        Ok(Arg::Value("val2")),
        Ok(Arg::ShortOption('v')),
        Ok(Arg::ShortOption('s')),
    ])]
    #[case("arg1 --option1 -- val1 -f val2 -vs", &[
        Ok(Arg::Value("arg1")),
        Ok(Arg::LongOption("option1")),
        Ok(Arg::DoubleDash),
        Ok(Arg::Value("val1")),
        Ok(Arg::Value("-f")),
        Ok(Arg::Value("val2")),
        Ok(Arg::Value("-vs")),
    ])]
    #[case("arg1 -бjв", &[
        Ok(Arg::Value("arg1")),
        Err(ArgError::NonAsciiShortOption),
        Err(ArgError::NonAsciiShortOption),
        Ok(Arg::ShortOption('j')),
        Err(ArgError::NonAsciiShortOption),
        Err(ArgError::NonAsciiShortOption),
    ])]
    fn arg_tokens(#[case] input: &str, #[case] expected: &[Result<Arg<'_>, ArgError>]) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();
        let args = ArgList::new(tokens);
        let mut iter = args.args();

        for arg in expected {
            let actual = iter.next().unwrap();
            assert_eq!(&actual, arg);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[rstest]
    #[case(r#"arg1 "arg2 long" arg3"#, &["arg1", "arg2 long", "arg3"])]
    #[case("  ", &[])]
    #[case(r#""""#, &[""])]
    fn test_iter(#[case] input: &str, #[case] expected: &[&str]) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();
        let args = ArgList::new(tokens);
        let mut iter = args.iter();

        for &arg in expected {
            assert_eq!(iter.next(), Some(arg));
        }
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_eq() {
        let mut input = b"arg1 arg2".to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();
        let args1 = ArgList::new(tokens);

        let mut input = b"   arg1    arg2  ".to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();
        let args2 = ArgList::new(tokens);

        assert_eq!(args1, args2)
    }
}
