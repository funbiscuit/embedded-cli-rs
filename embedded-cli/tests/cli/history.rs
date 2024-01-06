use crate::wrapper::{CliWrapper, OwnedCommand};

use crate::terminal::assert_terminal;

#[test]
fn navigation() {
    let mut cli = CliWrapper::new();

    cli.process_str("abc");
    cli.send_enter();
    cli.process_str("test1");
    cli.send_enter();
    cli.process_str("def");
    cli.send_enter();

    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        5,
        vec!["$ abc", "$ test1", "$ def", "$ def"]
    );

    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        7,
        vec!["$ abc", "$ test1", "$ def", "$ test1"]
    );

    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        5,
        vec!["$ abc", "$ test1", "$ def", "$ abc"]
    );

    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        5,
        vec!["$ abc", "$ test1", "$ def", "$ abc"]
    );

    cli.send_down();
    assert_terminal!(
        cli.terminal(),
        7,
        vec!["$ abc", "$ test1", "$ def", "$ test1"]
    );

    cli.send_down();
    assert_terminal!(
        cli.terminal(),
        5,
        vec!["$ abc", "$ test1", "$ def", "$ def"]
    );

    cli.send_down();
    assert_terminal!(cli.terminal(), 2, vec!["$ abc", "$ test1", "$ def", "$"]);

    cli.send_up();
    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        7,
        vec!["$ abc", "$ test1", "$ def", "$ test1"]
    );

    cli.send_enter();
    assert_terminal!(
        cli.terminal(),
        2,
        vec!["$ abc", "$ test1", "$ def", "$ test1", "$"]
    );
    assert_eq!(
        cli.received_commands().last().unwrap(),
        &OwnedCommand {
            name: "test1".to_string(),
            args: vec![],
        }
    );
}

#[test]
fn modify_when_in_history() {
    let mut cli = CliWrapper::new();

    cli.process_str("abc");
    cli.send_enter();
    cli.process_str("test1");
    cli.send_enter();
    cli.process_str("def");
    cli.send_enter();

    cli.send_up();
    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        7,
        vec!["$ abc", "$ test1", "$ def", "$ test1"]
    );

    cli.send_backspace();
    cli.send_backspace();
    cli.process_str("a");

    assert_terminal!(
        cli.terminal(),
        6,
        vec!["$ abc", "$ test1", "$ def", "$ tesa"]
    );

    cli.send_up();
    assert_terminal!(
        cli.terminal(),
        5,
        vec!["$ abc", "$ test1", "$ def", "$ abc"]
    );

    cli.send_down();
    assert_terminal!(
        cli.terminal(),
        7,
        vec!["$ abc", "$ test1", "$ def", "$ test1"]
    );
}
