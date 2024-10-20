use crate::{
    arguments::{Args, FromArgError},
    autocomplete::Autocomplete,
    help::Help,
};

#[cfg(feature = "autocomplete")]
use crate::autocomplete::{Autocompletion, Request};

#[cfg(feature = "help")]
use {crate::help::HelpError, embedded_io::Write};

pub trait FromCommand<'a>: Sized {
    /// Parse command name and args into typed container
    fn parse(name: &'a str, args: Args<'a>) -> Result<Self, ParseError<'a>>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ParseError<'a> {
    MissingRequiredArgument {
        /// Name of the argument. For example `<FILE>`, `-f <FILE>`, `--file <FILE>`
        name: &'a str,
    },

    ParseValueError {
        value: &'a str,
        expected: &'static str,
    },

    UnexpectedArgument {
        value: &'a str,
    },

    UnexpectedLongOption {
        name: &'a str,
    },

    UnexpectedShortOption {
        name: char,
    },

    UnknownCommand,
}

impl<'a> From<FromArgError<'a>> for ParseError<'a> {
    fn from(error: FromArgError<'a>) -> Self {
        Self::ParseValueError {
            value: error.value,
            expected: error.expected,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawCommand<'a> {
    /// Name of the command.
    ///
    /// In `set led 1 1` name is `set`
    name: &'a str,

    /// Argument list of the command
    ///
    /// In `set led 1 1` arguments is `led 1 1`
    args: Args<'a>,
}

impl<'a> RawCommand<'a> {
    pub fn new(name: &'a str, args: Args<'a>) -> Self {
        Self { name, args }
    }

    pub fn args(&self) -> Args<'a> {
        self.args.clone()
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a> Autocomplete for RawCommand<'a> {
    #[cfg(feature = "autocomplete")]
    fn autocomplete(_: Request<'_>, _: &mut Autocompletion<'_>) {
        // noop
    }
}

impl<'a> Help for RawCommand<'a> {
    #[cfg(feature = "help")]
    fn command_count() -> usize {
        0
    }

    #[cfg(feature = "help")]
    fn list_commands<W: Write<Error = E>, E: embedded_io::Error>(
        _: &mut crate::writer::Writer<'_, W, E>,
    ) -> Result<(), E> {
        // noop
        Ok(())
    }

    #[cfg(feature = "help")]
    fn command_help<
        W: Write<Error = E>,
        E: embedded_io::Error,
        F: FnMut(&mut crate::writer::Writer<'_, W, E>) -> Result<(), E>,
    >(
        _: &mut F,
        _: &str,
        _: Args<'_>,
        _: &mut crate::writer::Writer<'_, W, E>,
    ) -> Result<(), HelpError<E>> {
        // noop
        Err(HelpError::UnknownCommand)
    }
}

impl<'a> FromCommand<'a> for RawCommand<'a> {
    fn parse(name: &'a str, args: Args<'a>) -> Result<Self, ParseError<'a>> {
        Ok(RawCommand { name, args })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::command::FromCommand;
    use crate::{arguments::Args, command::RawCommand, token::Tokens};

    #[rstest]
    #[case("set led 1", "set", "led 1")]
    #[case("  get   led   2  ", "get", "led   2")]
    #[case("get", "get", "")]
    #[case("set led 1", "set", "led 1")]
    fn parsing_some(#[case] input: &str, #[case] name: &str, #[case] args: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let input_tokens = Tokens::new(input);
        let (input_name, input_tokens) = input_tokens.split_first().unwrap();
        let input_args = Args::new(input_tokens);
        let mut args = args.as_bytes().to_vec();
        let args = core::str::from_utf8_mut(&mut args).unwrap();
        let arg_tokens = Tokens::new(args);

        assert_eq!(
            RawCommand::parse(input_name, input_args).unwrap(),
            RawCommand {
                name: name,
                args: Args::new(arg_tokens)
            }
        );
    }
}
