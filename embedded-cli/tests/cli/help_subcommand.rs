use embedded_cli::Command;
use rstest::rstest;

use crate::impl_convert;
use crate::wrapper::CliWrapper;

use crate::terminal::assert_terminal;

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase<'a> {
    /// Base command
    #[command(name = "base1")]
    Base1 {
        /// Optional argument
        #[arg(short, long)]
        name: Option<&'a str>,

        /// Some level
        #[arg(short, long)]
        level: u8,

        /// Make things verbose
        #[arg(short)]
        verbose: bool,

        #[command(subcommand)]
        command: CliBase1Sub<'a>,
    },

    /// Another base command
    #[command(name = "base2", subcommand)]
    Base2(CliBase2Sub<'a>),
}

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase1Sub<'a> {
    /// Get something
    Get {
        /// Optional item
        #[arg(short, long)]
        item: Option<&'a str>,

        /// Another verbose flag
        #[arg(short, long)]
        verbose: bool,

        #[command(subcommand)]
        command: CliBase1SubSub<'a>,
    },
    /// Set something
    Set {
        /// Another required value
        value: &'a str,
    },
}

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase1SubSub<'a> {
    /// Command something
    Cmd {
        /// Very optional item
        #[arg(short, long)]
        item: Option<&'a str>,

        /// Third verbose flag
        #[arg(short, long)]
        verbose: bool,

        /// Required positional
        file: &'a str,
    },
    /// Test something
    Test {
        /// Test verbose flag
        #[arg(short, long)]
        verbose: bool,

        /// Tested required value
        value: &'a str,
    },
}

#[derive(Debug, Clone, Command, PartialEq)]
enum CliBase2Sub<'a> {
    /// Get something but differently
    Get {
        /// Also optional item
        #[arg(short, long)]
        item: Option<&'a str>,

        /// Third verbose flag
        #[arg(short, long)]
        verbose: bool,

        /// Required file
        file: &'a str,
    },
    /// Write something
    Write {
        /// Required line to write
        line: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base {
    Base1 {
        name: Option<String>,

        level: u8,

        verbose: bool,

        command: Base1Sub,
    },
    Base2(Base2Sub),
}

#[derive(Debug, Clone, PartialEq)]
enum Base1Sub {
    Get {
        item: Option<String>,

        verbose: bool,

        command: Base1SubSub,
    },
    Set {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base1SubSub {
    Cmd {
        item: Option<String>,

        verbose: bool,

        file: String,
    },
    Test {
        verbose: bool,

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
                level,
                verbose,
                command,
            } => Self::Base1 {
                name: name.map(|n| n.to_string()),
                level,
                verbose,
                command: command.into(),
            },
            CliBase::Base2(command) => Self::Base2(command.into()),
        }
    }
}

impl<'a> From<CliBase1Sub<'a>> for Base1Sub {
    fn from(value: CliBase1Sub<'a>) -> Self {
        match value {
            CliBase1Sub::Get {
                item,
                verbose,
                command,
            } => Self::Get {
                item: item.map(|n| n.to_string()),
                verbose,
                command: command.into(),
            },
            CliBase1Sub::Set { value } => Self::Set {
                value: value.to_string(),
            },
        }
    }
}

impl<'a> From<CliBase1SubSub<'a>> for Base1SubSub {
    fn from(value: CliBase1SubSub<'a>) -> Self {
        match value {
            CliBase1SubSub::Cmd {
                item,
                verbose,
                file,
            } => Self::Cmd {
                item: item.map(|n| n.to_string()),
                verbose,
                file: file.into(),
            },
            CliBase1SubSub::Test { verbose, value } => Self::Test {
                verbose,
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
#[case("base1 --help", &[
    "Base command",
    "",
    "Usage: base1 [OPTIONS] <COMMAND>",
    "",
    "Options:",
    "  -n, --name [NAME]    Optional argument",
    "  -l, --level <LEVEL>  Some level",
    "  -v                   Make things verbose",
    "  -h, --help           Print help",
    "",
    "Commands:",
    "  get  Get something",
    "  set  Set something",
])]
#[case("base1 get --help", &[
    "Get something",
    "", 
    "Usage: base1 get [OPTIONS] <COMMAND>",
    "",
    "Options:",
    "  -i, --item [ITEM]  Optional item",
    "  -v, --verbose      Another verbose flag",
    "  -h, --help         Print help",
    "",
    "Commands:",
    "  cmd   Command something",
    "  test  Test something",
])]
#[case("base1 --name some -v get --help", &[
    "Get something",
    "", 
    "Usage: base1 get [OPTIONS] <COMMAND>",
    "",
    "Options:",
    "  -i, --item [ITEM]  Optional item",
    "  -v, --verbose      Another verbose flag",
    "  -h, --help         Print help",
    "",
    "Commands:",
    "  cmd   Command something",
    "  test  Test something",
])]
#[case("base1 set --help", &[
    "Set something",
    "",
    "Usage: base1 set <VALUE>",
    "",
    "Arguments:",
    "  <VALUE>  Another required value",
    "",
    "Options:",
    "  -h, --help  Print help",
])]
#[case("base1 get cmd --help", &[
    "Command something",
    "",
    "Usage: base1 get cmd [OPTIONS] <FILE>",
    "",
    "Arguments:",
    "  <FILE>  Required positional",
    "",
    "Options:",
    "  -i, --item [ITEM]  Very optional item",
    "  -v, --verbose      Third verbose flag",
    "  -h, --help         Print help",
])]
#[case("base1 --name some -v get --verbose cmd --help", &[
    "Command something",
    "",
    "Usage: base1 get cmd [OPTIONS] <FILE>",
    "",
    "Arguments:",
    "  <FILE>  Required positional",
    "",
    "Options:",
    "  -i, --item [ITEM]  Very optional item",
    "  -v, --verbose      Third verbose flag",
    "  -h, --help         Print help",
])]
#[case("base1 get test --help", &[
    "Test something",
    "",
    "Usage: base1 get test [OPTIONS] <VALUE>",
    "",
    "Arguments:",
    "  <VALUE>  Tested required value",
    "",
    "Options:",
    "  -v, --verbose  Test verbose flag",
    "  -h, --help     Print help",
])]
#[case("base2 --help", &[
    "Another base command",
    "",
    "Usage: base2 <COMMAND>",
    "",
    "Options:",
    "  -h, --help  Print help",
    "",
    "Commands:",
    "  get    Get something but differently",
    "  write  Write something",
])]
#[case("base2 get --help", &[
    "Get something but differently",
    "",
    "Usage: base2 get [OPTIONS] <FILE>",
    "",
    "Arguments:",
    "  <FILE>  Required file",
    "",
    "Options:",
    "  -i, --item [ITEM]  Also optional item",
    "  -v, --verbose      Third verbose flag",
    "  -h, --help         Print help",
])]
#[case("base2 write --help", &[
    "Write something",
    "",
    "Usage: base2 write <LINE>",
    "",
    "Arguments:",
    "  <LINE>  Required line to write",
    "",
    "Options:",
    "  -h, --help  Print help",
])]
fn help(#[case] command: &str, #[case] expected: &[&str]) {
    let mut cli = CliWrapper::<Base>::new();
    let all_lines = [format!("$ {}", command)]
        .into_iter()
        .chain(expected.iter().map(|s| s.to_string()))
        .chain(Some("$".to_string()))
        .collect::<Vec<_>>();

    cli.process_str(command);

    cli.send_enter();

    assert_terminal!(cli.terminal(), 2, all_lines);

    assert!(cli.received_commands().is_empty());
}
