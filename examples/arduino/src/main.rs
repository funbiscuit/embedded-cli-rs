#![warn(rust_2018_idioms)]
#![no_std]
#![no_main]

use core::convert::Infallible;

use arduino_hal::hal::port;
use arduino_hal::pac::USART0;
use arduino_hal::port::mode;
use arduino_hal::port::Pin;
use arduino_hal::prelude::*;
use arduino_hal::usart::UsartWriter;
use avr_progmem::progmem_str as F;
use embedded_cli::cli::CliBuilder;
use embedded_cli::cli::CliEvent;
use embedded_cli::cli::CliHandle;
use embedded_cli::Command;
use embedded_io::ErrorType;
use panic_halt as _;
use ufmt::uwrite;
use ufmt::uwriteln;

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

/// Wrapper around usart so we can impl embedded_io::Write
/// which is required for cli
struct Writer(UsartWriter<USART0, Pin<mode::Input, port::PD0>, Pin<mode::Output, port::PD1>>);

impl ErrorType for Writer {
    type Error = Infallible;
}

impl embedded_io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for &b in buf {
            nb::block!(self.0.write(b)).void_unwrap();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        nb::block!(self.0.flush()).void_unwrap();
        Ok(())
    }
}

struct AppState {
    led_brightness: [u8; 4],
    num_commands: usize,
}

fn on_led(
    cli: &mut CliHandle<'_, Writer, Infallible>,
    state: &mut AppState,
    id: u8,
    command: LedCommand,
) -> Result<(), Infallible> {
    state.num_commands += 1;

    if id as usize > state.led_brightness.len() {
        uwrite!(cli.writer(), "{}{}{}", F!("LED"), id, F!(" not found"))?;
    } else {
        match command {
            LedCommand::Get => {
                uwrite!(
                    cli.writer(),
                    "{}{}{}{}",
                    F!("Current LED"),
                    id,
                    F!(" brightness: "),
                    state.led_brightness[id as usize]
                )?;
            }
            LedCommand::Set { value } => {
                state.led_brightness[id as usize] = value;
                uwrite!(
                    cli.writer(),
                    "{}{}{}{}",
                    F!("Setting LED"),
                    id,
                    F!(" brightness to "),
                    state.led_brightness[id as usize]
                )?;
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
                cli.writer().write_str(F!("Performing sampling with "))?;
                cli.writer().write_str(sampler)?;
                uwriteln!(
                    cli.writer(),
                    "{}{}{}",
                    F!("\nUsing "),
                    samples,
                    F!(" samples")
                )?;
            }
            uwrite!(
                cli.writer(),
                "{}{}{}{}",
                F!("Current ADC"),
                id,
                F!(" readings: "),
                43
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
    uwriteln!(cli.writer(), "{}{}", F!("Received: "), state.num_commands)?;
    Ok(())
}

#[arduino_hal::entry]
fn main() -> ! {
    try_run();

    // if run failed, stop execution
    panic!()
}

fn try_run() -> Option<()> {
    let dp = arduino_hal::Peripherals::take()?;
    let pins = arduino_hal::pins!(dp);

    let mut led = pins.d13.into_output();
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let (mut rx, tx) = serial.split();

    let writer = Writer(tx);

    led.set_low();

    // create static buffers for use in cli (so we're not using stack memory)
    // History buffer is 1 byte longer so max command fits in it (it requires extra byte at end)
    // SAFETY: buffers are passed to cli and are used by cli only
    let (command_buffer, history_buffer) = unsafe {
        static mut COMMAND_BUFFER: [u8; 40] = [0; 40];
        static mut HISTORY_BUFFER: [u8; 41] = [0; 41];
        #[allow(static_mut_refs)]
        (COMMAND_BUFFER.as_mut(), HISTORY_BUFFER.as_mut())
    };
    let mut cli = CliBuilder::default()
        .writer(writer)
        .command_buffer(command_buffer)
        .history_buffer(history_buffer)
        .build()
        .ok()?;

    // Create global state, that will be used for entire application
    let mut state = AppState {
        led_brightness: [0; 4],
        num_commands: 0,
    };

    let _ = cli.write(|writer| {
        // storing big text in progmem
        // for small text it's usually better to use normal &str literals
        uwrite!(
            writer,
            "{}",
            F!("Cli is running.
Type \"help\" for a list of commands.
Use backspace and tab to remove chars and autocomplete.
Use up and down for history navigation.
Use left and right to move inside input.")
        )?;
        Ok(())
    });

    loop {
        arduino_hal::delay_ms(10);
        led.toggle();

        let byte = nb::block!(rx.read()).void_unwrap();
        // Process incoming byte and poll new event (if happened)
        // Command type is specified for autocompletion, help and parsing
        // We can use different command type with each call to poll
        if let Ok(Some(mut event)) = cli.poll::<BaseCommand<'_>>(byte) {
            let _ = match event {
                CliEvent::Command(command, ref mut cli) => match command {
                    BaseCommand::Led { id, command } => on_led(cli, &mut state, id, command),
                    BaseCommand::Adc { id, command } => on_adc(cli, &mut state, id, command),
                    BaseCommand::Status => on_status(cli, &mut state),
                },
            };
        }
    }
}
