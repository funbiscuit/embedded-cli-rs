use crate::wrapper::{CliWrapper, RawCommand};

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

    cli.process_str("\n");
    assert_terminal!(cli.terminal(), 2, vec!["$ set led", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "set".to_string(),
            args: vec!["led".to_string()],
        })]
    );
}

#[test]
fn delete_with_backspace() {
    let mut cli = CliWrapper::default();

    cli.process_str("set");

    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.process_str("\x08\x08");

    assert_terminal!(cli.terminal(), 3, vec!["$ s"]);

    cli.process_str("\x08\x08\x08");

    assert_terminal!(cli.terminal(), 2, vec!["$"]);
}
