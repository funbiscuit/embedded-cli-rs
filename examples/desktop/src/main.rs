extern crate termion;

use embedded_cli::cli::{CliBuilder, CliHandle};
use embedded_cli::codes;
use embedded_cli::command::RawCommand;
use embedded_cli::{Command, CommandGroup};
use embedded_io::{ErrorType, Write};
use std::convert::Infallible;
use std::io::{stdin, stdout, Stdout, Write as _};
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use ufmt::{uwrite, uwriteln};

#[derive(Debug, Command)]
enum Base<'a> {
    /// Say hello to World or someone else
    Hello {
        /// To whom to say hello (World by default)
        name: Option<&'a str>,
    },

    /// Stop CLI and exit
    Exit,
}

#[derive(Debug, Command)]
#[command(help_title = "Manage Hardware")]
enum GetCommand {
    // By default command name is generated from variant name,
    // converting it to kebab case (get-led in this case)
    /// Get current LED value
    GetLed {
        /// ID of requested LED
        led: u8,
    },

    // Name can be specified explicitly
    /// Get current ADC value
    #[command(name = "getAdc")]
    GetAdc {
        /// ID of requested ADC
        adc: u8,
    },
}

#[derive(Debug, CommandGroup)]
enum Group<'a> {
    Base(Base<'a>),
    Get(GetCommand),
    Other(RawCommand<'a>),
}

pub struct Writer {
    stdout: RawTerminal<Stdout>,
}

impl ErrorType for Writer {
    type Error = Infallible;
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.stdout.write_all(buf).unwrap();
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.stdout.flush().unwrap();
        Ok(())
    }
}

struct AppState {
    num_commands: usize,
    should_exit: bool,
}

fn on_get(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    command: GetCommand,
) -> Result<(), Infallible> {
    state.num_commands += 1;

    match command {
        GetCommand::GetLed { led } => {
            uwrite!(
                cli.writer(),
                "Current LED{} brightness: {}",
                led,
                rand::random::<u8>()
            )?;
        }
        GetCommand::GetAdc { adc } => {
            uwrite!(
                cli.writer(),
                "Current ADC{} readings: {}",
                adc,
                rand::random::<u8>()
            )?;
        }
    }
    Ok(())
}

fn on_command(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    command: Base<'_>,
) -> Result<(), Infallible> {
    state.num_commands += 1;

    match command {
        Base::Hello { name } => {
            uwrite!(cli.writer(), "Hello, {}", name.unwrap_or("World"))?;
        }
        Base::Exit => {
            cli.writer().write_str("Cli will shutdown now")?;
            state.should_exit = true;
        }
    }
    Ok(())
}

fn on_unknown(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    command: RawCommand<'_>,
) -> Result<(), Infallible> {
    state.num_commands += 1;
    uwriteln!(cli.writer(), "Received command: {}", command.name())?;
    for (i, arg) in command.args().iter().enumerate() {
        uwriteln!(cli.writer(), "Argument {}: '{}'", i, arg)?;
    }
    uwriteln!(cli.writer(), "Total received: {}", state.num_commands)?;
    Ok(())
}

fn main() {
    let stdout = stdout().into_raw_mode().unwrap();

    let writer = Writer { stdout };

    let mut cli = CliBuilder::default().writer(writer).build().unwrap();

    // Create global state, that will be used for entire application
    let mut state = AppState {
        num_commands: 0,
        should_exit: false,
    };

    cli.write(|writer| {
        writer.writeln_str("Cli is running. Press 'Esc' to exit")?;
        writer.writeln_str(r#"Type "help" for a list of commands"#)?;
        writer.writeln_str("Use backspace and tab to remove chars and autocomplete")?;
        Ok(())
    })
    .unwrap();

    let stdin = stdin();
    for c in stdin.events() {
        let evt = c.unwrap();
        let bytes = match evt {
            Event::Key(Key::Esc) => break,
            Event::Key(Key::Up) => vec![codes::ESCAPE, b'[', b'A'],
            Event::Key(Key::Down) => vec![codes::ESCAPE, b'[', b'B'],
            Event::Key(Key::BackTab) => vec![codes::TABULATION],
            Event::Key(Key::Backspace) => vec![codes::BACKSPACE],
            Event::Key(Key::Char(c)) => {
                let mut buf = [0; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
            _ => continue,
        };
        // Process incoming byte
        // Command type is specified for autocompletion and help
        // Processor accepts closure where we can process parsed command
        // we can use different command and processor with each call
        // TODO: add example of login that uses different states
        for byte in bytes {
            cli.process_byte::<Group, _>(
                byte,
                &mut Group::processor(|cli, command| {
                    match command {
                        Group::Base(cmd) => on_command(cli, &mut state, cmd)?,
                        Group::Get(cmd) => on_get(cli, &mut state, cmd)?,
                        Group::Other(cmd) => on_unknown(cli, &mut state, cmd)?,
                    }
                    Ok(())
                }),
            )
            .unwrap();
        }

        if state.should_exit {
            break;
        }
    }
}
