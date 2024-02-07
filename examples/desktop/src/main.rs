#![warn(rust_2018_idioms)]

use embedded_cli::cli::{CliBuilder, CliHandle};
use embedded_cli::codes;
use embedded_cli::Command;
use embedded_io::{ErrorType, Write};
use std::convert::Infallible;
use std::io::{stdin, stdout, Stdout, Write as _};
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use ufmt::{uwrite, uwriteln};

#[derive(Debug, Command)]
enum BaseCommand<'a> {
    /// Control LEDs
    Led {
        /// LED id
        #[arg(long)]
        id: u8,

        #[command(subcommand)]
        command: LedCommand,
    },

    /// Control ADC
    Adc {
        /// ADC id
        #[arg(long)]
        id: u8,

        #[command(subcommand)]
        command: AdcCommand<'a>,
    },

    /// Show some status
    Status,

    /// Stop CLI and exit
    Exit,
}

#[derive(Debug, Command)]
enum LedCommand {
    /// Get current LED value
    Get,

    /// Set LED value
    Set {
        /// LED brightness
        value: u8,
    },
}

#[derive(Debug, Command)]
enum AdcCommand<'a> {
    /// Read ADC value
    Read {
        /// Print extra info
        #[arg(short = 'V', long)]
        verbose: bool,

        /// Sample count (16 by default)
        #[arg(long)]
        samples: Option<u8>,

        #[arg(long)]
        sampler: &'a str,
    },
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
    led_brightness: [u8; 4],
    num_commands: usize,
    should_exit: bool,
}

fn on_led(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    id: u8,
    command: LedCommand,
) -> Result<(), Infallible> {
    state.num_commands += 1;

    if id as usize > state.led_brightness.len() {
        uwrite!(cli.writer(), "LED{} not found", id)?;
    } else {
        match command {
            LedCommand::Get => {
                uwrite!(
                    cli.writer(),
                    "Current LED{} brightness: {}",
                    id,
                    state.led_brightness[id as usize]
                )?;
            }
            LedCommand::Set { value } => {
                state.led_brightness[id as usize] = value;
                uwrite!(cli.writer(), "Setting LED{} brightness to {}", id, value)?;
            }
        }
    }

    Ok(())
}

fn on_adc(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    id: u8,
    command: AdcCommand<'_>,
) -> Result<(), Infallible> {
    state.num_commands += 1;

    match command {
        AdcCommand::Read {
            verbose,
            samples,
            sampler,
        } => {
            let samples = samples.unwrap_or(16);
            if verbose {
                cli.writer().write_str("Performing sampling with ")?;
                cli.writer().write_str(sampler)?;
                uwriteln!(cli.writer(), "\nUsing {} samples", samples)?;
            }
            uwrite!(
                cli.writer(),
                "Current ADC{} readings: {}",
                id,
                rand::random::<u8>()
            )?;
        }
    }
    Ok(())
}

fn on_status(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
) -> Result<(), Infallible> {
    state.num_commands += 1;
    uwriteln!(cli.writer(), "Received: {}", state.num_commands)?;
    Ok(())
}

fn main() {
    let stdout = stdout().into_raw_mode().unwrap();

    let writer = Writer { stdout };

    let mut cli = CliBuilder::default().writer(writer).build().unwrap();

    // Create global state, that will be used for entire application
    let mut state = AppState {
        led_brightness: rand::random(),
        num_commands: 0,
        should_exit: false,
    };

    cli.write(|writer| {
        uwrite!(
            writer,
            "Cli is running. Press 'Esc' to exit
Type \"help\" for a list of commands.
Use backspace and tab to remove chars and autocomplete.
Use up and down for history navigation.
Use left and right to move inside input."
        )?;
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
            Event::Key(Key::Right) => vec![codes::ESCAPE, b'[', b'C'],
            Event::Key(Key::Left) => vec![codes::ESCAPE, b'[', b'D'],
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
            cli.process_byte::<BaseCommand<'_>, _>(
                byte,
                &mut BaseCommand::processor(|cli, command| match command {
                    BaseCommand::Led { id, command } => on_led(cli, &mut state, id, command),
                    BaseCommand::Adc { id, command } => on_adc(cli, &mut state, id, command),
                    BaseCommand::Status => on_status(cli, &mut state),
                    BaseCommand::Exit => {
                        state.should_exit = true;
                        cli.writer().write_str("Cli will shutdown now")
                    }
                }),
            )
            .unwrap();
        }

        if state.should_exit {
            break;
        }
    }
}
