use embedded_cli::Command;
use rstest::rstest;

use crate::impl_convert;
use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase<'a> {
    #[command(name = "base1")]
    Base1 {
        #[arg(short, long)]
        name: Option<&'a str>,

        #[arg(short)]
        verbose: bool,

        #[command(subcommand)]
        command: CliBase1Sub<'a>,
    },
    #[command(name = "base2")]
    Base2 {
        #[arg(short, long)]
        level: u8,

        #[command(subcommand)]
        command: CliBase2Sub<'a>,
    },
}

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase1Sub<'a> {
    Get {
        #[arg(short, long)]
        item: Option<&'a str>,

        #[arg(short, long)]
        verbose: bool,

        file: &'a str,
    },
    Set {
        value: &'a str,
    },
}

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase2Sub<'a> {
    Get {
        #[arg(short, long)]
        item: Option<&'a str>,

        #[arg(short, long)]
        verbose: bool,

        file: &'a str,
    },
    Write {
        line: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base {
    Base1 {
        name: Option<String>,

        verbose: bool,

        command: Base1Sub,
    },
    Base2 {
        level: u8,

        command: Base2Sub,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base1Sub {
    Get {
        item: Option<String>,

        verbose: bool,

        file: String,
    },
    Set {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base2Sub {
    Get {
        item: Option<String>,

        verbose: bool,

        file: String,
    },
    Write {
        line: String,
    },
}

impl_convert! {CliBase<'_> => Base, command, { command.into() }}

impl<'a> From<CliBase<'a>> for Base {
    fn from(value: CliBase<'a>) -> Self {
        match value {
            CliBase::Base1 {
                name,
                verbose,
                command,
            } => Self::Base1 {
                name: name.map(|n| n.to_string()),
                verbose,
                command: command.into(),
            },
            CliBase::Base2 { level, command } => Self::Base2 {
                level,
                command: command.into(),
            },
        }
    }
}

impl<'a> From<CliBase1Sub<'a>> for Base1Sub {
    fn from(value: CliBase1Sub<'a>) -> Self {
        match value {
            CliBase1Sub::Get {
                item,
                verbose,
                file,
            } => Self::Get {
                item: item.map(|n| n.to_string()),
                verbose,
                file: file.to_string(),
            },
            CliBase1Sub::Set { value } => Self::Set {
                value: value.to_string(),
            },
        }
    }
}

impl<'a> From<CliBase2Sub<'a>> for Base2Sub {
    fn from(value: CliBase2Sub<'a>) -> Self {
        match value {
            CliBase2Sub::Get {
                item,
                verbose,
                file,
            } => Self::Get {
                item: item.map(|n| n.to_string()),
                verbose,
                file: file.to_string(),
            },
            CliBase2Sub::Write { line } => Self::Write {
                line: line.to_string(),
            },
        }
    }
}

#[rstest]
#[case("base1 --name test-name -v get --item config -v some-file", Base::Base1 {
    name: Some("test-name".to_string()),
    verbose: true,
    command: Base1Sub::Get {
        item: Some("config".to_string()),
        verbose: true,
        file: "some-file".to_string(),
    }
})]
#[case("base1 -v --name test-name set some-value", Base::Base1 {
    name: Some("test-name".to_string()),
    verbose: true,
    command: Base1Sub::Set {
        value: "some-value".to_string(),
    }
})]
#[case("base2 --level 23 get --item config -v some-file", Base::Base2 {
    level: 23,
    command: Base2Sub::Get {
        item: Some("config".to_string()),
        verbose: true,
        file: "some-file".to_string(),
    }
})]
#[case("base2 --level 23 write lines", Base::Base2 {
    level: 23,
    command: Base2Sub::Write {
        line: "lines".to_string(),
    }
})]
fn options_parsing(#[case] command: &str, #[case] expected: Base) {
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
