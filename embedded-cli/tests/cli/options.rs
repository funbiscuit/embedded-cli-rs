use embedded_cli::Command;
use rstest::rstest;

use crate::impl_convert;
use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum CliTestCommand<'a> {
    Cmd {
        #[arg(short, long)]
        name: Option<&'a str>,

        #[arg(long = "conf")]
        config: &'a str,

        #[arg(short)]
        level: u8,

        #[arg(short = 'V', long)]
        verbose: bool,

        file: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum TestCommand {
    Cmd {
        name: Option<String>,
        config: String,
        level: u8,
        verbose: bool,
        file: String,
    },
}

impl_convert! {CliTestCommand<'_> => TestCommand, command, {
    match command {
        cmd => cmd.into(),
    }
}}

impl<'a> From<CliTestCommand<'a>> for TestCommand {
    fn from(value: CliTestCommand<'a>) -> Self {
        match value {
            CliTestCommand::Cmd {
                name,
                config,
                level,
                verbose,
                file,
            } => Self::Cmd {
                name: name.map(|n| n.to_string()),
                config: config.to_string(),
                level,
                verbose,
                file: file.to_string(),
            },
        }
    }
}

#[rstest]
#[case("cmd --name test-name --conf config -l 5 -V some-file", TestCommand::Cmd {
    name: Some("test-name".to_string()),
    config: "config".to_string(),
    level: 5,
    verbose: true,
    file: "some-file".to_string(),
})]
#[case("cmd --conf config -l 35 --verbose some-file", TestCommand::Cmd {
    name: None,
    config: "config".to_string(),
    level: 35,
    verbose: true,
    file: "some-file".to_string(),
})]
#[case("cmd --conf conf2 file -n name2 -Vl 25", TestCommand::Cmd {
    name: Some("name2".to_string()),
    config: "conf2".to_string(),
    level: 25,
    verbose: true,
    file: "file".to_string(),
})]
#[case("cmd file3 --conf conf3 -l 17", TestCommand::Cmd {
    name: None,
    config: "conf3".to_string(),
    level: 17,
    verbose: false,
    file: "file3".to_string(),
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
