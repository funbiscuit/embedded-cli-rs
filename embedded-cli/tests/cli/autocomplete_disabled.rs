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
fn autocomplete_disabled() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("e");

    cli.send_tab();

    assert_terminal!(cli.terminal(), 3, vec!["$ e"]);
}
