use embedded_cli::command::RawCommand;
use embedded_cli::service::FromRaw;
use embedded_cli::Command;

use crate::wrapper::{CliWrapper, CommandConvert, ParseError};

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum TestCommand {
    GetLed,
    GetAdc,
    Exit,
}

impl CommandConvert for TestCommand {
    fn convert(cmd: RawCommand<'_>) -> Result<Self, ParseError> {
        Ok(TestCommand::parse(cmd)?)
    }
}

#[test]
fn complete_when_single_variant() {
    let mut cli = CliWrapper::new();

    cli.process_str("e\t");

    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.process_str("\n");

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);

    assert_terminal!(cli.terminal(), 2, vec!["$ exit", "$"]);
}

#[test]
fn complete_when_multiple_variants() {
    let mut cli = CliWrapper::new();

    cli.process_str("g\t");

    assert_terminal!(cli.terminal(), 6, vec!["$ get-"]);

    cli.process_str("a\t");

    assert_terminal!(cli.terminal(), 10, vec!["$ get-adc"]);

    cli.process_str("\n");
    assert_terminal!(cli.terminal(), 2, vec!["$ get-adc", "$"]);
    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::GetAdc)]);
}

#[test]
fn complete_when_name_finished() {
    let mut cli = CliWrapper::new();

    cli.process_str("exit\t");

    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.process_str("\n");

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_with_leading_spaces() {
    let mut cli = CliWrapper::new();

    cli.process_str("  ex\t");

    assert_terminal!(cli.terminal(), 9, vec!["$   exit"]);

    cli.process_str("\n");

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_with_trailing_spaces() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("ex \t");

    assert_terminal!(cli.terminal(), 5, vec!["$ ex"]);
}
