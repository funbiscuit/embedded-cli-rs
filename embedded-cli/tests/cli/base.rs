use crate::wrapper::{Arg, CliWrapper, RawCommand};

use crate::terminal::assert_terminal;

#[test]
fn simple_input() {
    let mut cli = CliWrapper::default();

    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.process_str("set");

    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.process_str(" led");

    assert_terminal!(cli.terminal(), 9, vec!["$ set led"]);

    assert!(cli.received_commands().is_empty());

    cli.send_enter();
    assert_terminal!(cli.terminal(), 2, vec!["$ set led", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "set".to_string(),
            args: vec![Ok(Arg::Value("led".to_string()))],
        })]
    );
}

#[test]
fn delete_with_backspace() {
    let mut cli = CliWrapper::default();

    cli.process_str("set");

    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.send_backspace();

    assert_terminal!(cli.terminal(), 4, vec!["$ se"]);

    cli.send_backspace();

    assert_terminal!(cli.terminal(), 3, vec!["$ s"]);

    cli.send_backspace();
    cli.send_backspace();
    cli.send_backspace();

    assert_terminal!(cli.terminal(), 2, vec!["$"]);
}

#[test]
fn move_insert() {
    let mut cli = CliWrapper::default();

    cli.process_str("set");
    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.send_left();
    assert_terminal!(cli.terminal(), 4, vec!["$ set"]);

    cli.send_left();
    assert_terminal!(cli.terminal(), 3, vec!["$ set"]);

    cli.process_str("up-d");
    assert_terminal!(cli.terminal(), 7, vec!["$ sup-det"]);

    cli.send_backspace();
    assert_terminal!(cli.terminal(), 6, vec!["$ sup-et"]);

    cli.send_right();
    assert_terminal!(cli.terminal(), 7, vec!["$ sup-et"]);

    cli.process_str("d");
    assert_terminal!(cli.terminal(), 8, vec!["$ sup-edt"]);

    cli.send_enter();
    assert_terminal!(cli.terminal(), 2, vec!["$ sup-edt", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "sup-edt".to_string(),
            args: vec![],
        })]
    );
}

#[test]
fn try_move_outside() {
    let mut cli = CliWrapper::default();

    cli.process_str("set");
    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.send_right();
    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.send_left();
    cli.send_left();
    cli.send_left();
    assert_terminal!(cli.terminal(), 2, vec!["$ set"]);

    cli.send_left();
    assert_terminal!(cli.terminal(), 2, vec!["$ set"]);

    cli.process_str("d-");
    assert_terminal!(cli.terminal(), 4, vec!["$ d-set"]);

    cli.send_enter();
    assert_terminal!(cli.terminal(), 2, vec!["$ d-set", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "d-set".to_string(),
            args: vec![],
        })]
    );
}
