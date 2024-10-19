use embedded_cli::Command;

use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum TestCommand {
    GetLed,
    GetAdc,
    Exit,
}

#[test]
fn complete_when_single_variant() {
    let mut cli = CliWrapper::new();

    cli.process_str("e");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_enter();

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);

    assert_terminal!(cli.terminal(), 2, vec!["$ exit", "$"]);
}

#[test]
fn complete_when_multiple_variants() {
    let mut cli = CliWrapper::new();

    cli.process_str("g");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 6, vec!["$ get-"]);

    cli.process_str("a");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 10, vec!["$ get-adc"]);

    cli.send_enter();
    assert_terminal!(cli.terminal(), 2, vec!["$ get-adc", "$"]);
    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::GetAdc)]);
}

#[test]
fn complete_when_name_finished() {
    let mut cli = CliWrapper::new();

    cli.process_str("exit");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_enter();

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_with_leading_spaces() {
    let mut cli = CliWrapper::new();

    cli.process_str("  ex");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 9, vec!["$   exit"]);

    cli.send_enter();

    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_with_trailing_spaces() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("ex ");
    cli.send_tab();

    assert_terminal!(cli.terminal(), 5, vec!["$ ex"]);
}

#[test]
fn complete_when_inside() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("ex");
    cli.send_left();
    assert_terminal!(cli.terminal(), 3, vec!["$ ex"]);

    cli.send_tab();
    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_enter();
    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_when_inside_with_trailing_spaces() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("ex ");
    cli.send_left();
    cli.send_left();
    assert_terminal!(cli.terminal(), 3, vec!["$ ex"]);

    cli.send_tab();
    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_enter();
    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_when_inside_after_complete() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("e");
    cli.send_tab();
    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_left();
    cli.send_left();
    cli.send_left();
    assert_terminal!(cli.terminal(), 4, vec!["$ exit"]);

    cli.send_tab();
    assert_terminal!(cli.terminal(), 7, vec!["$ exit"]);

    cli.send_enter();
    assert_eq!(cli.received_commands(), vec![Ok(TestCommand::Exit)]);
}

#[test]
fn complete_when_inside_without_variants() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("do");
    cli.send_left();
    assert_terminal!(cli.terminal(), 3, vec!["$ do"]);

    cli.send_tab();
    assert_terminal!(cli.terminal(), 3, vec!["$ do"]);
}

#[test]
fn complete_when_inside_and_empty_completion() {
    let mut cli = CliWrapper::<TestCommand>::new();

    cli.process_str("g");
    cli.send_tab();
    assert_terminal!(cli.terminal(), 6, vec!["$ get-"]);

    cli.send_left();
    cli.send_left();
    assert_terminal!(cli.terminal(), 4, vec!["$ get-"]);

    cli.send_tab();
    assert_terminal!(cli.terminal(), 6, vec!["$ get-"]);
}
