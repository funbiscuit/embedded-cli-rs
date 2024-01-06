use crate::wrapper::{CliWrapper, OwnedCommand};

use crate::terminal::assert_terminal;

#[test]
fn complete_when_single_variant() {
    let mut cli = CliWrapper::new();
    cli.set_known_commands(&["hello", "get-led", "get-adc"]);

    cli.process_str("h\t");

    assert_terminal!(cli.terminal(), 8, vec!["$ hello"]);

    cli.process_str("\n");

    assert_eq!(
        cli.received_commands(),
        vec![OwnedCommand {
            name: "hello".to_string(),
            args: vec![],
        }]
    );

    assert_terminal!(cli.terminal(), 2, vec!["$ hello", "$"]);
}

#[test]
fn complete_when_multiple_variants() {
    let mut cli = CliWrapper::new();
    cli.set_known_commands(&["hello", "get-led", "get-adc"]);

    cli.process_str("g\t");

    assert_terminal!(cli.terminal(), 6, vec!["$ get-"]);

    cli.process_str("a\t");

    assert_terminal!(cli.terminal(), 10, vec!["$ get-adc"]);

    cli.process_str("\n");
    assert_terminal!(cli.terminal(), 2, vec!["$ get-adc", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![OwnedCommand {
            name: "get-adc".to_string(),
            args: vec![],
        }]
    );
}

#[test]
fn complete_when_name_finished() {
    let mut cli = CliWrapper::new();
    cli.set_known_commands(&["hello", "get-led", "get-adc"]);

    cli.process_str("hello\t");

    assert_terminal!(cli.terminal(), 8, vec!["$ hello"]);

    cli.process_str("\n");

    assert_eq!(
        cli.received_commands(),
        vec![OwnedCommand {
            name: "hello".to_string(),
            args: vec![],
        }]
    );
}

#[test]
fn complete_with_leading_spaces() {
    let mut cli = CliWrapper::new();
    cli.set_known_commands(&["hello", "get-led", "get-adc"]);

    cli.process_str("  he\t");

    assert_terminal!(cli.terminal(), 10, vec!["$   hello"]);

    cli.process_str("\n");

    assert_eq!(
        cli.received_commands(),
        vec![OwnedCommand {
            name: "hello".to_string(),
            args: vec![],
        }]
    );
}

#[test]
fn complete_with_trailing_spaces() {
    let mut cli = CliWrapper::new();
    cli.set_known_commands(&["hello", "get-led", "get-adc"]);

    cli.process_str("he \t");

    assert_terminal!(cli.terminal(), 5, vec!["$ he"]);
}
