use std::{cell::RefCell, convert::Infallible, fmt::Debug, marker::PhantomData, rc::Rc};

use embedded_cli::{
    arguments::Arg as CliArg,
    autocomplete::Autocomplete,
    cli::{Cli, CliBuilder, CliEvent, CliHandle},
    command::{FromCommand, ParseError as CliParseError, RawCommand as CliRawCommand},
    help::Help,
};
use embedded_io::ErrorType;

use crate::terminal::Terminal;

/// Helper macro to impl all Command traits for some new type
/// when impls already exist for other type and we have impl From<other type> for new type
#[macro_export]
macro_rules! impl_convert {
    ($from_ty:ty => $to_ty:ty) => {
        impl embedded_cli::autocomplete::Autocomplete for $to_ty {
            #[cfg(feature = "autocomplete")]
            fn autocomplete(
                request: embedded_cli::autocomplete::Request<'_>,
                autocompletion: &mut embedded_cli::autocomplete::Autocompletion<'_>,
            ) {
                <$from_ty>::autocomplete(request, autocompletion)
            }
        }

        impl embedded_cli::help::Help for $to_ty {
            #[cfg(feature = "help")]
            fn command_count() -> usize {
                <$from_ty>::command_count()
            }

            #[cfg(feature = "help")]
            fn list_commands<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
                writer: &mut embedded_cli::writer::Writer<'_, W, E>,
            ) -> Result<(), E> {
                <$from_ty>::list_commands(writer)
            }

            #[cfg(feature = "help")]
            fn command_help<
                W: embedded_io::Write<Error = E>,
                E: embedded_io::Error,
                F: FnMut(&mut embedded_cli::writer::Writer<'_, W, E>) -> Result<(), E>,
            >(
                parent: &mut F,
                name: &str,
                args: embedded_cli::arguments::Args<'_>,
                writer: &mut embedded_cli::writer::Writer<'_, W, E>,
            ) -> Result<(), embedded_cli::help::HelpError<E>> {
                <$from_ty>::command_help(parent, name, args, writer)
            }
        }

        impl<'a> embedded_cli::command::FromCommand<'a> for $to_ty {
            fn parse(
                name: &'a str,
                args: embedded_cli::arguments::Args<'a>,
            ) -> Result<Self, embedded_cli::command::ParseError<'a>> {
                let parsed = <$from_ty as embedded_cli::command::FromCommand>::parse(name, args)?;
                let cmd = parsed.into();
                Ok(cmd)
            }
        }
    };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Arg {
    DoubleDash,
    LongOption(String),
    ShortOption(char),
    Value(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawCommand {
    pub name: String,
    pub args: Vec<Arg>,
}

impl_convert! { CliRawCommand<'_> => RawCommand }

impl<'a> From<CliRawCommand<'a>> for RawCommand {
    fn from(value: CliRawCommand<'a>) -> Self {
        Self {
            name: value.name().to_string(),
            args: value
                .args()
                .iter()
                .map(|arg| match arg {
                    CliArg::DoubleDash => Arg::DoubleDash,
                    CliArg::LongOption(name) => Arg::LongOption(name.to_string()),
                    CliArg::ShortOption(name) => Arg::ShortOption(name),
                    CliArg::Value(value) => Arg::Value(value.to_string()),
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct State<T> {
    written: Vec<u8>,
    commands: Vec<Result<T, ParseError>>,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            written: Default::default(),
            commands: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    MissingRequiredArgument { name: String },

    ParseValueError { value: String, expected: String },

    UnexpectedArgument { value: String },

    UnexpectedLongOption { name: String },

    UnexpectedShortOption { name: char },

    UnknownCommand,
    Other,
}

impl<'a> From<CliParseError<'a>> for ParseError {
    fn from(value: CliParseError<'a>) -> Self {
        match value {
            CliParseError::MissingRequiredArgument { name } => {
                Self::MissingRequiredArgument { name: name.into() }
            }
            CliParseError::ParseValueError { value, expected } => Self::ParseValueError {
                value: value.into(),
                expected: expected.into(),
            },
            CliParseError::UnexpectedArgument { value } => Self::UnexpectedArgument {
                value: value.into(),
            },
            CliParseError::UnexpectedLongOption { name } => {
                Self::UnexpectedLongOption { name: name.into() }
            }
            CliParseError::UnexpectedShortOption { name } => {
                Self::UnexpectedShortOption { name: name.into() }
            }
            CliParseError::UnknownCommand => Self::UnknownCommand,
            _ => Self::Other,
        }
    }
}

pub struct CliWrapper<T: Autocomplete + Help + Clone> {
    /// Actual cli object
    cli: Cli<Writer<T>, Infallible, &'static mut [u8], &'static mut [u8]>,

    handler: Option<
        Box<dyn FnMut(&mut CliHandle<'_, Writer<T>, Infallible>, T) -> Result<(), Infallible>>,
    >,

    state: Rc<RefCell<State<T>>>,

    terminal: Terminal,
}

impl Default for CliWrapper<RawCommand> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Autocomplete + Help + Clone> CliWrapper<T> {
    pub fn builder() -> CliWrapperBuilder<T> {
        CliWrapperBuilder {
            command_size: 80,
            history_size: 500,
            prompt: None,
            _ph: PhantomData,
        }
    }

    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn process_str(&mut self, text: &str)
    where
        T: for<'c> FromCommand<'c>,
    {
        for b in text.as_bytes() {
            if let Some(mut event) = self.cli.poll::<T>(*b).unwrap() {
                match event {
                    CliEvent::Command(command, ref mut cli) => {
                        self.state.borrow_mut().commands.push(Ok(command.clone()));
                        if let Some(handler) = &mut self.handler {
                            handler(cli, command).unwrap();
                        }
                    }
                }
            }
        }

        self.update_terminal();
    }

    pub fn send_backspace(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\x08")
    }

    pub fn send_down(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\x1B[B")
    }

    pub fn send_enter(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\n")
    }

    pub fn send_left(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\x1B[D")
    }

    pub fn send_right(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\x1B[C")
    }

    pub fn send_tab(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\t")
    }

    pub fn send_up(&mut self)
    where
        T: for<'c> FromCommand<'c>,
    {
        self.process_str("\x1B[A")
    }

    pub fn set_handler(
        &mut self,
        handler: impl FnMut(&mut CliHandle<'_, Writer<T>, Infallible>, T) -> Result<(), Infallible>
            + 'static,
    ) {
        self.handler = Some(Box::new(handler));
    }

    pub fn set_prompt(&mut self, prompt: &'static str) {
        self.cli.set_prompt(prompt).unwrap();
        self.update_terminal();
    }

    pub fn received_commands(&self) -> Vec<Result<T, ParseError>> {
        self.state.borrow().commands.to_vec()
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn write_str(&mut self, text: &str) {
        self.cli.write(|writer| writer.write_str(text)).unwrap();
        self.update_terminal();
    }

    fn update_terminal(&mut self) {
        for byte in self.state.borrow_mut().written.drain(..) {
            self.terminal.receive_byte(byte)
        }
    }
}

#[derive(Debug)]
pub struct CliWrapperBuilder<T: Autocomplete + Help + Clone> {
    command_size: usize,
    history_size: usize,
    prompt: Option<&'static str>,
    _ph: PhantomData<T>,
}

impl<T: Autocomplete + Help + Clone> CliWrapperBuilder<T> {
    pub fn build(self) -> CliWrapper<T> {
        let state = Rc::new(RefCell::new(State::default()));

        let writer = Writer {
            state: state.clone(),
        };

        //TODO: impl Buffer for Vec so no need to leak
        let builder = CliBuilder::default()
            .writer(writer)
            .command_buffer(vec![0; self.command_size].leak())
            .history_buffer(vec![0; self.history_size].leak());
        let builder = if let Some(prompt) = self.prompt {
            builder.prompt(prompt)
        } else {
            builder
        };
        let cli = builder.build().unwrap();

        let terminal = Terminal::new();
        let mut wrapper = CliWrapper {
            cli,
            handler: None,
            state,
            terminal,
        };
        wrapper.update_terminal();
        wrapper
    }

    pub fn prompt(mut self, prompt: &'static str) -> Self {
        self.prompt = Some(prompt);
        self
    }
}

pub struct Writer<T> {
    state: Rc<RefCell<State<T>>>,
}

impl<T> ErrorType for Writer<T> {
    type Error = Infallible;
}

impl<T> embedded_io::Write for Writer<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.state.borrow_mut().written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
