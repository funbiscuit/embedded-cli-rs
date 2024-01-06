use core::marker::PhantomData;

use embedded_io::Write;

use crate::{
    arguments::ArgList,
    autocomplete::{Autocompletion, Request},
    cli::CliHandle,
    service::{Autocomplete, CommandProcessor, FromRaw, Help, HelpError, ParseError, ProcessError},
    token::Tokens,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawCommand<'a> {
    /// Name of the command.
    ///
    /// In `set led 1 1` name is `set`
    name: &'a str,

    /// Argument list of the command
    ///
    /// In `set led 1 1` arguments is `led 1 1`
    args: ArgList<'a>,
}

impl<'a> RawCommand<'a> {
    /// Crate raw command from input tokens
    pub(crate) fn from_tokens(mut tokens: Tokens<'_>) -> Option<RawCommand<'_>> {
        let name = tokens.remove(0)?;

        Some(RawCommand {
            name,
            args: ArgList::new(tokens),
        })
    }

    pub fn args(&self) -> ArgList<'a> {
        self.args.clone()
    }

    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn processor<
        W: Write<Error = E>,
        E: embedded_io::Error,
        F: FnMut(&mut CliHandle<'_, W, E>, RawCommand<'_>) -> Result<(), E>,
    >(
        f: F,
    ) -> impl CommandProcessor<W, E> {
        struct Processor<
            W: Write<Error = E>,
            E: embedded_io::Error,
            F: FnMut(&mut CliHandle<'_, W, E>, RawCommand<'_>) -> Result<(), E>,
        > {
            f: F,
            _ph: PhantomData<(W, E)>,
        }

        impl<
                W: Write<Error = E>,
                E: embedded_io::Error,
                F: FnMut(&mut CliHandle<'_, W, E>, RawCommand<'_>) -> Result<(), E>,
            > CommandProcessor<W, E> for Processor<W, E, F>
        {
            fn process<'a>(
                &mut self,
                cli: &mut CliHandle<'_, W, E>,
                raw: RawCommand<'a>,
            ) -> Result<(), ProcessError<'a, E>> {
                (self.f)(cli, raw)?;
                Ok(())
            }
        }

        Processor {
            f,
            _ph: PhantomData,
        }
    }
}

impl<'a> Autocomplete for RawCommand<'a> {
    fn autocomplete(_: Request<'_>, _: &mut Autocompletion<'_>) {
        // noop
    }
}

impl<'a> Help for RawCommand<'a> {
    fn help<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
        _: crate::help::HelpRequest<'_>,
        _: &mut crate::writer::Writer<'_, W, E>,
    ) -> Result<(), crate::service::HelpError<E>> {
        // noop
        Err(HelpError::UnknownCommand)
    }
}

impl<'a> FromRaw<'a> for RawCommand<'a> {
    fn parse(raw: RawCommand<'a>) -> Result<Self, ParseError<'a>> {
        Ok(raw)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{arguments::ArgList, command::RawCommand, token::Tokens};

    #[rstest]
    #[case("set led 1", "set", "led 1")]
    #[case("  get   led   2  ", "get", "led   2")]
    #[case("get", "get", "")]
    #[case("set led 1", "set", "led 1")]
    fn parsing_some(#[case] input: &str, #[case] name: &str, #[case] args: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let input_tokens = Tokens::new(input).unwrap();
        let mut args = args.as_bytes().to_vec();
        let args = core::str::from_utf8_mut(&mut args).unwrap();
        let arg_tokens = Tokens::new(args).unwrap();

        assert_eq!(
            RawCommand::from_tokens(input_tokens).unwrap(),
            RawCommand {
                name: name,
                args: ArgList::new(arg_tokens)
            }
        );
    }

    #[rstest]
    #[case("   ")]
    #[case("")]
    fn parsing_none(#[case] input: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();

        assert!(RawCommand::from_tokens(tokens).is_none());
    }
}
