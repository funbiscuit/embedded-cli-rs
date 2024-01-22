use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[test]
fn history_disabled() {
    let mut cli = CliWrapper::default();

    cli.process_str("abc");
    cli.send_enter();
    cli.process_str("test1");
    cli.send_enter();
    cli.process_str("def");
    cli.send_enter();

    cli.send_up();
    assert_terminal!(cli.terminal(), 2, vec!["$ abc", "$ test1", "$ def", "$"]);

    cli.send_down();
    assert_terminal!(cli.terminal(), 2, vec!["$ abc", "$ test1", "$ def", "$"]);
}
