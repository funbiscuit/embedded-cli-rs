#![no_std]
#![no_main]

use core::convert::Infallible;

use arduino_hal::hal::port;
use arduino_hal::pac::USART0;
use arduino_hal::port::mode;
use arduino_hal::port::Pin;
use arduino_hal::prelude::_void_ResultVoidExt;
use arduino_hal::usart::UsartWriter;
use avr_progmem::progmem_str as F;
use embedded_cli::cli::CliBuilder;
use embedded_cli::cli::CliHandle;
use embedded_cli::command::RawCommand;
use embedded_cli::{Command, CommandGroup};
use embedded_hal::serial::Read;
use embedded_hal::serial::Write;
use embedded_io::ErrorType;
use panic_halt as _;
use ufmt::uwrite;
use ufmt::uwriteln;

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
    num_commands: usize,
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
                "{}{}{}{}",
                F!("Current LED"),
                led,
                F!(" brightness: "),
                12
            )?;
        }
        GetCommand::GetAdc { adc } => {
            uwrite!(
                cli.writer(),
                "{}{}{}{}",
                F!("Current ADC"),
                adc,
                F!(" readings: "),
                23
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
            // last write in command callback may or may not
            // end with newline. so both uwrite!() and uwriteln!()
            // will give identical results
            uwrite!(cli.writer(), "{}{}", F!("Hello, "), name.unwrap_or("World"))?;
        }
        Base::Exit => {
            // We can write via normal function if formatting not needed
            cli.writer().write_str(F!("Cli can't shutdown now"))?;
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
    // Use writeln to write separate lines
    uwriteln!(
        cli.writer(),
        "{}{}",
        F!("Received command: "),
        command.name()
    )?;
    for (i, arg) in command.args().iter().enumerate() {
        uwriteln!(
            cli.writer(),
            "{}{}{}{}'",
            F!("Argument "),
            i,
            F!(": '"),
            arg
        )?;
    }
    uwriteln!(
        cli.writer(),
        "{}{}",
        F!("Total received: "),
        state.num_commands
    )?;
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
    // SAFETY: buffers are passed to cli and are used by cli only
    let (command_buffer, history_buffer) = unsafe {
        static mut COMMAND_BUFFER: [u8; 32] = [0; 32];
        static mut HISTORY_BUFFER: [u8; 32] = [0; 32];
        (COMMAND_BUFFER.as_mut(), HISTORY_BUFFER.as_mut())
    };
    let mut cli = CliBuilder::default()
        .writer(writer)
        .command_buffer(command_buffer)
        .history_buffer(history_buffer)
        .build()
        .ok()?;

    // Create global state, that will be used for entire application
    let mut state = AppState { num_commands: 0 };

    let _ = cli.write(|writer| {
        // storing big text in progmem
        // for small text it's usually better to use normal &str literals
        uwrite!(
            writer,
            "{}",
            F!("Cli is running.
Type \"help\" for a list of commands.
Use backspace and tab to remove chars and autocomplete.
Use up and down for history navigation")
        )?;
        Ok(())
    });

    loop {
        arduino_hal::delay_ms(10);
        led.toggle();

        let byte = nb::block!(rx.read()).void_unwrap();
        // Process incoming byte
        // Command type is specified for autocompletion and help
        // Processor accepts closure where we can process parsed command
        // we can use different command and processor with each call
        let _ = cli.process_byte::<Group, _>(
            byte,
            &mut Group::processor(|cli, command| {
                match command {
                    Group::Base(cmd) => on_command(cli, &mut state, cmd)?,
                    Group::Get(cmd) => on_get(cli, &mut state, cmd)?,
                    Group::Other(cmd) => on_unknown(cli, &mut state, cmd)?,
                }
                Ok(())
            }),
        );
    }
}
