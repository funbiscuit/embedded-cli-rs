use embedded_cli::Command;
use rstest::rstest;

use crate::impl_convert;
use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum CliTestCommand<'a> {
    Cmd {
        #[arg(long, default_value = "default name")]
        name: &'a str,

        #[arg(long, default_value = "8")]
        level: u8,

        #[arg(long, default_value_t = 9)]
        level2: u8,

        #[arg(long, default_value_t)]
        level3: u8,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum TestCommand {
    Cmd {
        name: String,
        level: u8,
        level2: u8,
        level3: u8,
    },
}

impl_convert! { CliTestCommand<'_> => TestCommand }

impl<'a> From<CliTestCommand<'a>> for TestCommand {
    fn from(value: CliTestCommand<'a>) -> Self {
        match value {
            CliTestCommand::Cmd {
                name,
                level,
                level2,
                level3,
            } => Self::Cmd {
                name: name.to_string(),
                level,
                level2,
                level3,
            },
        }
    }
}

#[rstest]
#[case("cmd --name test-name --level 1 --level2 2 --level3 3", TestCommand::Cmd {
    name: "test-name".to_string(),
    level: 1,
    level2: 2,
    level3: 3,
})]
#[case("cmd", TestCommand::Cmd {
    name: "default name".to_string(),
    level: 8,
    level2: 9,
    level3: 0,
})]
fn options_parsing(#[case] command: &str, #[case] expected: TestCommand) {
    let mut cli = CliWrapper::new();

    cli.process_str(command);

    cli.send_enter();

    assert_terminal!(
        cli.terminal(),
        2,
        vec![format!("$ {}", command), "$".to_string()]
    );

    assert_eq!(cli.received_commands(), vec![Ok(expected)]);
}
