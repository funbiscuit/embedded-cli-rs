use crate::token::{Tokens, TokensIter};

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

    pub fn iter(&self) -> impl Iterator<Item = &'a str> {
        TokensIter::new(self.args, self.empty)
    }
}

impl<'a> PartialEq for ArgList<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
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
