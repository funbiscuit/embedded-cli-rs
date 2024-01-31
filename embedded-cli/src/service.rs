use embedded_io::Write;

use crate::{cli::CliHandle, command::RawCommand};

#[cfg(feature = "autocomplete")]
use crate::autocomplete::{Autocompletion, Request};

#[cfg(feature = "help")]
use crate::writer::Writer;

#[derive(Debug)]
pub enum ProcessError<'a, E: embedded_io::Error> {
    ParseError(ParseError<'a>),
    WriteError(E),
}

#[derive(Debug)]
pub enum ParseError<'a> {
    NotEnoughArguments,
    Other(&'a str),
    ParseArgumentError { value: &'a str },
    TooManyArguments { expected: usize },
    UnknownFlag { flag: char },
    UnknownOption { name: &'a str },
    UnknownCommand,
}

impl<'a, E: embedded_io::Error> From<E> for ProcessError<'a, E> {
    fn from(value: E) -> Self {
        Self::WriteError(value)
    }
}

impl<'a, E: embedded_io::Error> From<ParseError<'a>> for ProcessError<'a, E> {
    fn from(value: ParseError<'a>) -> Self {
        Self::ParseError(value)
    }
}

#[derive(Debug)]
pub enum HelpError<E: embedded_io::Error> {
    WriteError(E),
    UnknownCommand,
}

impl<E: embedded_io::Error> From<E> for HelpError<E> {
    fn from(value: E) -> Self {
        Self::WriteError(value)
    }
}

pub trait Autocomplete {
    // trait is kept available so it's possible to use same where clause
    #[cfg(feature = "autocomplete")]
    /// Try to process autocompletion request
    /// Autocompleted bytes (not present in request) should be written to
    /// given autocompletion.
    fn autocomplete(request: Request<'_>, autocompletion: &mut Autocompletion<'_>);
}

// trait is kept available so it's possible to use same where clause
pub trait Help {
    #[cfg(feature = "help")]
    /// How many commands are known
    fn command_count() -> usize;

    #[cfg(feature = "help")]
    /// Print all commands and short description of each
    fn list_commands<W: Write<Error = E>, E: embedded_io::Error>(
        writer: &mut Writer<'_, W, E>,
    ) -> Result<(), E>;

    #[cfg(feature = "help")]
    /// Print help for given command. Command might contain -h or --help options
    /// Use given writer to print help text
    /// If help request cannot be processed by this object,
    /// Err(HelpError::UnknownCommand) must be returned
    fn command_help<
        W: Write<Error = E>,
        E: embedded_io::Error,
        F: FnMut(&mut Writer<'_, W, E>) -> Result<(), E>,
    >(
        parent: &mut F,
        command: RawCommand<'_>,
        writer: &mut Writer<'_, W, E>,
    ) -> Result<(), HelpError<E>>;
}

pub trait FromRaw<'a>: Sized {
    /// Parse raw command into typed command
    fn parse(raw: RawCommand<'a>) -> Result<Self, ParseError<'a>>;
}

pub trait CommandProcessor<W: Write<Error = E>, E: embedded_io::Error> {
    fn process<'a>(
        &mut self,
        cli: &mut CliHandle<'_, W, E>,
        raw: RawCommand<'a>,
    ) -> Result<(), ProcessError<'a, E>>;
}

impl<W, E, F> CommandProcessor<W, E> for F
where
    W: Write<Error = E>,
    E: embedded_io::Error,
    F: for<'a> FnMut(&mut CliHandle<'_, W, E>, RawCommand<'a>) -> Result<(), ProcessError<'a, E>>,
{
    fn process<'a>(
        &mut self,
        cli: &mut CliHandle<'_, W, E>,
        command: RawCommand<'a>,
    ) -> Result<(), ProcessError<'a, E>> {
        self(cli, command)
    }
}
