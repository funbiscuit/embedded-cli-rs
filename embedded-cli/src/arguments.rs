use crate::{
    token::{Tokens, TokensIter},
    utils,
};

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

    /// Short option. Only single UTF-8 char is stored (without `-`).
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
pub struct Args<'a> {
    tokens: Tokens<'a>,
}

impl<'a> Args<'a> {
    /// Create new arg list from given tokens
    pub fn new(tokens: Tokens<'a>) -> Self {
        Self { tokens }
    }

    /// Iterate parsed values of argumens as `Arg` values
    pub fn iter(&self) -> ArgsIter<'a> {
        ArgsIter::new(self.tokens.iter())
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Iterate over raw values of argumens as `&str` values
    pub fn iter_raw(&self) -> TokensIter<'a> {
        self.tokens.iter()
    }

    /// Peek first value of argumen as `&str` value (without removing it from args)
    pub fn peek_raw(&self) -> Option<&'a str> {
        self.tokens.iter().next()
    }
}

impl<'a> PartialEq for Args<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.iter_raw().eq(other.iter_raw())
    }
}

#[derive(Clone, Debug)]
pub struct ArgsIter<'a> {
    values_only: bool,

    /// Short options (utf8 chars) that
    /// are left from previous iteration
    leftover: &'a str,

    tokens: TokensIter<'a>,
}

impl<'a> ArgsIter<'a> {
    fn new(tokens: TokensIter<'a>) -> Self {
        Self {
            values_only: false,
            leftover: "",
            tokens,
        }
    }

    /// Converts whats left in this iterator back to `Args`
    ///
    /// If iterator was in the middle of iterating of collapsed
    /// short options (like `-vhs`), non iterated options are discarded
    pub fn into_args(self) -> Args<'a> {
        Args::new(self.tokens.into_tokens())
    }
}

impl<'a> Iterator for ArgsIter<'a> {
    type Item = Arg<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((opt, leftover)) = utils::char_pop_front(self.leftover) {
            self.leftover = leftover;
            return Some(Arg::ShortOption(opt));
        }

        let raw = self.tokens.next()?;
        let bytes = raw.as_bytes();

        if self.values_only {
            return Some(Arg::Value(raw));
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
                let (opt, leftover) =
                    unsafe { utils::char_pop_front(raw.get_unchecked(1..)).unwrap_unchecked() };
                self.leftover = leftover;

                return Some(Arg::ShortOption(opt));
            }
        } else {
            Arg::Value(raw)
        };

        Some(token)
    }
}

#[derive(Debug)]
pub struct FromArgError<'a> {
    pub value: &'a str,
    pub expected: &'static str,
}

pub trait FromArg<'a> {
    fn from_arg(arg: &'a str) -> Result<Self, FromArgError<'a>>
    where
        Self: Sized;
}

impl<'a> FromArg<'a> for &'a str {
    fn from_arg(arg: &'a str) -> Result<Self, FromArgError<'a>> {
        Ok(arg)
    }
}

macro_rules! impl_arg_fromstr {
    ($id:ident) => (
        impl<'a> FromArg<'a> for $id {
            fn from_arg(arg: &'a str) -> Result<Self, FromArgError<'a>> {
                arg.parse().map_err(|_| FromArgError {
                    value: arg,
                    expected: stringify!($id),
                })
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

    use crate::{arguments::Args, token::Tokens};

    use super::Arg;

    #[rstest]
    #[case("arg1 --option1 val1 -f val2 -vs", &[
        Arg::Value("arg1"),
        Arg::LongOption("option1"),
        Arg::Value("val1"),
        Arg::ShortOption('f'),
        Arg::Value("val2"),
        Arg::ShortOption('v'),
        Arg::ShortOption('s'),
    ])]
    #[case("arg1 --option1 -- val1 -f val2 -vs", &[
        Arg::Value("arg1"),
        Arg::LongOption("option1"),
        Arg::DoubleDash,
        Arg::Value("val1"),
        Arg::Value("-f"),
        Arg::Value("val2"),
        Arg::Value("-vs"),
    ])]
    #[case("arg1 -бj佗𑿌", &[
        Arg::Value("arg1"),
        Arg::ShortOption('б'),
        Arg::ShortOption('j'),
        Arg::ShortOption('佗'),
        Arg::ShortOption('𑿌'),
    ])]
    fn arg_tokens(#[case] input: &str, #[case] expected: &[Arg<'_>]) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let args = Args::new(tokens);
        let mut iter = args.iter();

        for arg in expected {
            let actual = iter.next().unwrap();
            assert_eq!(&actual, arg);
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_eq() {
        let mut input = b"arg1 arg2".to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let args1 = Args::new(tokens);

        let mut input = b"   arg1    arg2  ".to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let args2 = Args::new(tokens);

        assert_eq!(args1, args2)
    }
}
