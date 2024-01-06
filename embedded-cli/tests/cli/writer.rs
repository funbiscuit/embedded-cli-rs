use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[test]
fn write_external() {
    let mut cli = CliWrapper::new();

    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.write_str("test");

    assert_terminal!(cli.terminal(), 2, vec!["test", "$"]);

    cli.process_str("set");

    assert_terminal!(cli.terminal(), 5, vec!["test", "$ set"]);

    cli.write_str("abc");

    assert_terminal!(cli.terminal(), 5, vec!["test", "abc", "$ set"]);

    cli.write_str("def\r\n");

    assert_terminal!(cli.terminal(), 5, vec!["test", "abc", "def", "$ set"]);

    cli.write_str("gh\r\n\r\n");

    assert_terminal!(
        cli.terminal(),
        5,
        vec!["test", "abc", "def", "gh", "", "$ set"]
    );
}

#[test]
fn write_from_service() {
    let mut cli = CliWrapper::new();

    cli.set_handler(|cli, cmd| {
        cli.writer().write_str(r#"from command ""#)?;
        cli.writer().write_str(&cmd.name)?;
        cli.writer().writeln_str(r#"""#)?;
        cli.writer().write_str("another line")?;
        Ok(())
    });

    assert_terminal!(cli.terminal(), 2, vec!["$"]);

    cli.process_str("set 123\n");

    assert_terminal!(
        cli.terminal(),
        2,
        vec!["$ set 123", r#"from command "set""#, "another line", "$"]
    );
}
