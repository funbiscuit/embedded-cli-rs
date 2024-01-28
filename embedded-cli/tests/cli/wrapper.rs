use std::{cell::RefCell, convert::Infallible, fmt::Debug, rc::Rc};

use embedded_cli::{
    arguments::{Arg as CliArg, ArgError},
    cli::{Cli, CliBuilder, CliHandle},
    command::RawCommand as CliRawCommand,
    service::{Autocomplete, CommandProcessor, Help, ParseError as CliParseError, ProcessError},
};
use embedded_io::ErrorType;

use crate::terminal::Terminal;

/// Helper trait to wrap parsed command or error with lifetime into owned command
pub trait CommandConvert: Sized {
    fn convert(cmd: CliRawCommand<'_>) -> Result<Self, ParseError>;
}

#[macro_export]
macro_rules! impl_convert {
    ($from_ty:ty => $to_ty:ty, $var_name:ident, $conversion:block) => {
        impl embedded_cli::service::Autocomplete for $to_ty {
            #[cfg(feature = "autocomplete")]
            fn autocomplete(
                request: embedded_cli::autocomplete::Request<'_>,
                autocompletion: &mut embedded_cli::autocomplete::Autocompletion<'_>,
            ) {
                <$from_ty>::autocomplete(request, autocompletion)
            }
        }

        impl embedded_cli::service::Help for $to_ty {
            #[cfg(feature = "help")]
            fn help<W: embedded_io::Write<Error = E>, E: embedded_io::Error>(
                request: embedded_cli::help::HelpRequest<'_>,
                writer: &mut embedded_cli::writer::Writer<'_, W, E>,
            ) -> Result<(), embedded_cli::service::HelpError<E>> {
                <$from_ty>::help(request, writer)
            }
        }

        impl crate::wrapper::CommandConvert for $to_ty {
            fn convert(
                cmd: embedded_cli::command::RawCommand<'_>,
            ) -> Result<Self, crate::wrapper::ParseError> {
                let $var_name = <$from_ty as embedded_cli::service::FromRaw>::parse(cmd)?;
                let cmd = $conversion;
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
    pub args: Vec<Result<Arg, ArgError>>,
}

impl_convert! {CliRawCommand<'_> => RawCommand, command, {
    match command {
        cmd => cmd.into(),
    }
}}

impl<'a> From<CliRawCommand<'a>> for RawCommand {
    fn from(value: CliRawCommand<'a>) -> Self {
        Self {
            name: value.name().to_string(),
            args: value
                .args()
                .args()
                .map(|arg| match arg {
                    Ok(arg) => Ok(match arg {
                        CliArg::DoubleDash => Arg::DoubleDash,
                        CliArg::LongOption(name) => Arg::LongOption(name.to_string()),
                        CliArg::ShortOption(name) => Arg::ShortOption(name),
                        CliArg::Value(value) => Arg::Value(value.to_string()),
                    }),
                    Err(err) => Err(err),
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
    NotEnoughArguments,
    Other(String),
    ParseArgumentError { value: String },
    TooManyArguments { expected: usize },
    UnknownOption { name: String },
    UnknownFlag { flag: char },
    UnknownCommand,
}

impl<'a> From<CliParseError<'a>> for ParseError {
    fn from(value: CliParseError<'a>) -> Self {
        match value {
            CliParseError::NotEnoughArguments => ParseError::NotEnoughArguments,
            CliParseError::Other(s) => ParseError::Other(s.to_string()),
            CliParseError::ParseArgumentError { value } => ParseError::ParseArgumentError {
                value: value.to_string(),
            },
            CliParseError::TooManyArguments { expected } => {
                ParseError::TooManyArguments { expected }
            }
            CliParseError::UnknownOption { name } => ParseError::UnknownOption {
                name: name.to_string(),
            },
            CliParseError::UnknownFlag { flag } => ParseError::UnknownFlag { flag },
            CliParseError::UnknownCommand => ParseError::UnknownCommand,
        }
    }
}

pub struct CliWrapper<T: Autocomplete + Help + CommandConvert + Clone> {
    /// Actual cli object
    cli: Cli<Writer<T>, Infallible, &'static mut [u8], &'static mut [u8]>,

    handler: Option<
        Box<dyn FnMut(&mut CliHandle<'_, Writer<T>, Infallible>, T) -> Result<(), Infallible>>,
    >,

    state: Rc<RefCell<State<T>>>,

    terminal: Terminal,
}

struct App<T: CommandConvert + Clone> {
    handler: Option<
        Box<dyn FnMut(&mut CliHandle<'_, Writer<T>, Infallible>, T) -> Result<(), Infallible>>,
    >,
    state: Rc<RefCell<State<T>>>,
}

impl<T: CommandConvert + Clone> CommandProcessor<Writer<T>, Infallible> for App<T> {
    fn process<'a>(
        &mut self,
        cli: &mut CliHandle<'_, Writer<T>, Infallible>,
        command: CliRawCommand<'a>,
    ) -> Result<(), ProcessError<'a, Infallible>> {
        let command = T::convert(command);

        self.state.borrow_mut().commands.push(command.clone());
        if let (Some(handler), Ok(command)) = (&mut self.handler, command) {
            handler(cli, command)?;
        }
        Ok(())
    }
}

impl Default for CliWrapper<RawCommand> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Autocomplete + Help + CommandConvert + Clone> CliWrapper<T> {
    pub fn new() -> Self {
        Self::new_with_sizes(80, 500)
    }

    pub fn process_str(&mut self, text: &str) {
        let mut app = App {
            handler: self.handler.take(),
            state: self.state.clone(),
        };
        for b in text.as_bytes() {
            self.cli.process_byte::<T, _>(*b, &mut app).unwrap();
        }

        self.handler = app.handler.take();
        self.update_terminal();
    }

    pub fn send_backspace(&mut self) {
        self.process_str("\x08")
    }

    pub fn send_down(&mut self) {
        self.process_str("\x1B[B")
    }

    pub fn send_enter(&mut self) {
        self.process_str("\n")
    }

    pub fn send_left(&mut self) {
        self.process_str("\x1B[D")
    }

    pub fn send_right(&mut self) {
        self.process_str("\x1B[C")
    }

    pub fn send_tab(&mut self) {
        self.process_str("\t")
    }

    pub fn send_up(&mut self) {
        self.process_str("\x1B[A")
    }

    pub fn set_handler(
        &mut self,
        handler: impl FnMut(&mut CliHandle<'_, Writer<T>, Infallible>, T) -> Result<(), Infallible>
            + 'static,
    ) {
        self.handler = Some(Box::new(handler));
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

    fn new_with_sizes(command_size: usize, history_size: usize) -> Self {
        let state = Rc::new(RefCell::new(State::default()));

        let writer = Writer {
            state: state.clone(),
        };

        //TODO: impl Buffer for Vec so no need to leak
        let cli = CliBuilder::default()
            .writer(writer)
            .command_buffer(vec![0; command_size].leak())
            .history_buffer(vec![0; history_size].leak())
            .build()
            .unwrap();

        let terminal = Terminal::new();
        let mut wrapper = Self {
            cli,
            handler: None,
            state,
            terminal,
        };
        wrapper.update_terminal();
        wrapper
    }

    fn update_terminal(&mut self) {
        for byte in self.state.borrow_mut().written.drain(..) {
            self.terminal.receive_byte(byte)
        }
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
