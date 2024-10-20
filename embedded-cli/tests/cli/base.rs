use rstest::rstest;

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
            args: vec![Arg::Value("led".to_string())],
        })]
    );
}

#[test]
fn empty_input() {
    let mut cli = CliWrapper::default();

    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.send_enter();
    cli.send_enter();

    assert_terminal!(cli.terminal(), 2, vec!["$", "$", "$"]);

    cli.process_str("set led");

    assert_terminal!(cli.terminal(), 9, vec!["$", "$", "$ set led"]);

    assert!(cli.received_commands().is_empty());

    cli.send_enter();
    assert_terminal!(cli.terminal(), 2, vec!["$", "$", "$ set led", "$"]);
    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "set".to_string(),
            args: vec![Arg::Value("led".to_string())],
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

#[rstest]
#[case("#")]
#[case("###> ")]
#[case("")]
fn set_prompt_dynamic(#[case] prompt: &'static str) {
    let mut cli = CliWrapper::default();
    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.set_prompt(prompt);
    assert_terminal!(cli.terminal(), prompt.len(), vec![prompt.trim()]);

    cli.set_prompt("$ ");
    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.set_prompt(prompt);
    assert_terminal!(cli.terminal(), prompt.len(), vec![prompt.trim()]);

    cli.process_str("set");
    assert_terminal!(
        cli.terminal(),
        prompt.len() + 3,
        vec![format!("{}set", prompt)]
    );

    cli.set_prompt("$ ");
    assert_terminal!(cli.terminal(), 5, vec!["$ set"]);

    cli.set_handler(move |cli, _| {
        cli.set_prompt(prompt);
        Ok(())
    });
    cli.send_enter();
    assert_terminal!(cli.terminal(), prompt.len(), vec!["$ set", prompt.trim()]);

    cli.set_handler(move |cli, _| {
        cli.set_prompt("$ ");
        Ok(())
    });
    cli.process_str("get");
    cli.send_enter();
    assert_terminal!(
        cli.terminal(),
        2,
        vec![
            "$ set".to_string(),
            format!("{}get", prompt),
            "$".to_string()
        ]
    );

    assert_eq!(
        cli.received_commands(),
        vec![
            Ok(RawCommand {
                name: "set".to_string(),
                args: vec![],
            }),
            Ok(RawCommand {
                name: "get".to_string(),
                args: vec![],
            })
        ]
    );
}

#[rstest]
#[case("#")]
#[case("###> ")]
#[case("")]
fn set_prompt_static(#[case] prompt: &'static str) {
    let mut cli = CliWrapper::builder().prompt(prompt).build();
    assert_terminal!(cli.terminal(), prompt.len(), vec![prompt.trim_end()]);

    cli.process_str("set");
    assert_terminal!(
        cli.terminal(),
        prompt.len() + 3,
        vec![format!("{}set", prompt)]
    );

    cli.send_enter();
    assert_terminal!(
        cli.terminal(),
        prompt.len(),
        vec![format!("{}set", prompt), prompt.trim().to_string()]
    );

    assert_eq!(
        cli.received_commands(),
        vec![Ok(RawCommand {
            name: "set".to_string(),
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
