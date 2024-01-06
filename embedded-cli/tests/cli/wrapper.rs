use std::{cell::RefCell, convert::Infallible, fmt::Debug, rc::Rc};

use embedded_cli::{
    cli::{Cli, CliBuilder, CliHandle},
    command::RawCommand,
    service::{CommandProcessor, ProcessError},
};
use embedded_io::ErrorType;

use crate::terminal::Terminal;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedCommand {
    pub name: String,
    pub args: Vec<String>,
}

impl<'a> From<RawCommand<'a>> for OwnedCommand {
    fn from(value: RawCommand<'a>) -> Self {
        Self {
            name: value.name().to_string(),
            args: value.args().iter().map(str::to_string).collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    written: Vec<u8>,
    commands: Vec<OwnedCommand>,
}

pub struct CliWrapper {
    /// Actual cli object
    cli: Cli<Writer, Infallible, &'static mut [u8], &'static mut [u8]>,

    handler: Option<
        Box<
            dyn FnMut(
                &mut CliHandle<'_, Writer, Infallible>,
                OwnedCommand,
            ) -> Result<(), Infallible>,
        >,
    >,

    state: Rc<RefCell<State>>,

    terminal: Terminal,
}

struct App {
    handler: Option<
        Box<
            dyn FnMut(
                &mut CliHandle<'_, Writer, Infallible>,
                OwnedCommand,
            ) -> Result<(), Infallible>,
        >,
    >,
    state: Rc<RefCell<State>>,
}

impl CommandProcessor<Writer, Infallible> for App {
    fn process<'a>(
        &mut self,
        cli: &mut CliHandle<'_, Writer, Infallible>,
        command: RawCommand<'a>,
    ) -> Result<(), ProcessError<'a, Infallible>> {
        let command: OwnedCommand = command.into();
        self.state.borrow_mut().commands.push(command.clone());
        if let Some(ref mut handler) = self.handler {
            handler(cli, command)?;
        }
        Ok(())
    }
}

impl CliWrapper {
    pub fn new() -> Self {
        Self::new_with_sizes(80, 500)
    }

    pub fn process_str(&mut self, text: &str) {
        let mut app = App {
            handler: self.handler.take(),
            state: self.state.clone(),
        };
        for b in text.as_bytes() {
            self.cli
                .process_byte::<RawCommand<'_>, _>(*b, &mut app)
                .unwrap();
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

    pub fn send_up(&mut self) {
        self.process_str("\x1B[A")
    }

    pub fn set_handler(
        &mut self,
        handler: impl FnMut(&mut CliHandle<'_, Writer, Infallible>, OwnedCommand) -> Result<(), Infallible>
            + 'static,
    ) {
        self.handler = Some(Box::new(handler));
    }

    pub fn received_commands(&self) -> Vec<OwnedCommand> {
        self.state.borrow().commands.to_vec()
    }

    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    pub fn write_str(&mut self, text: &str) {
        self.cli.write(|writer| writer.write_str(text)).unwrap();
        self.update_terminal();
    }

    fn new_with_sizes(command_size: usize, history_size: usize) -> CliWrapper {
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
        let mut wrapper = CliWrapper {
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

pub struct Writer {
    state: Rc<RefCell<State>>,
}

impl ErrorType for Writer {
    type Error = Infallible;
}

impl embedded_io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.state.borrow_mut().written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
