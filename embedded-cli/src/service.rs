use embedded_io::Write;

use crate::{cli::CliHandle, command::RawCommand};

#[cfg(feature = "autocomplete")]
use crate::autocomplete::{Autocompletion, Request};

#[cfg(feature = "help")]
use crate::{help::HelpRequest, writer::Writer};

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

pub trait Help {
    /// Return length of longest command, contained in this service
    fn longest_command() -> usize {
        0
    }

    // trait is kept available so it's possible to use same where clause
    #[cfg(feature = "help")]
    /// Try to process help request
    /// Use given writer to print help text
    /// If help request cannot be processed by this service,
    /// Err(HelpError::UnknownCommand) must be returned
    fn help<W: Write<Error = E>, E: embedded_io::Error>(
        request: HelpRequest<'_>,
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
