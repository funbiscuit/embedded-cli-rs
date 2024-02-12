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

        /// Make things verbose
        #[arg(short)]
        verbose: bool,
    },

    /// Another base command
    #[command(name = "base2")]
    Base2 {
        /// Some level
        #[arg(short, long, value_name = "lvl")]
        level: u8,
    },

    /// Test command
    Test {
        /// Some task job
        #[arg(short = 'j', long = "job")]
        task: &'a str,

        /// Source file
        #[arg(value_name = "FILE")]
        file1: &'a str,

        /// Destination file
        file2: &'a str,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum Base {
    Base1 {
        name: Option<String>,
        verbose: bool,
    },
    Base2 {
        level: u8,
    },
    Test {
        task: String,

        file1: String,

        file2: String,
    },
}

impl_convert! {CliBase<'_> => Base, command, { command.into() }}

impl<'a> From<CliBase<'a>> for Base {
    fn from(value: CliBase<'a>) -> Self {
        match value {
            CliBase::Base1 { name, verbose } => Self::Base1 {
                name: name.map(|n| n.to_string()),
                verbose,
            },
            CliBase::Base2 { level } => Self::Base2 { level },
            CliBase::Test { task, file1, file2 } => Self::Test {
                task: task.to_string(),
                file1: file1.to_string(),
                file2: file2.to_string(),
            },
        }
    }
}

#[rstest]
#[case("base1 --help", &[
    "Base command",
    "",
    "Usage: base1 [OPTIONS]",
    "",
    "Options:",
    "  -n, --name [NAME]  Optional argument",
    "  -v                 Make things verbose",
    "  -h, --help         Print help",
])]
#[case("base1 -n name -v --help", &[
    "Base command",
    "",
    "Usage: base1 [OPTIONS]",
    "",
    "Options:",
    "  -n, --name [NAME]  Optional argument",
    "  -v                 Make things verbose",
    "  -h, --help         Print help",
])]
#[case("base2 --help", &[
    "Another base command",
    "",
    "Usage: base2 [OPTIONS]",
    "",
    "Options:",
    "  -l, --level <lvl>  Some level",
    "  -h, --help         Print help",
])]
#[case("test --help", &[
    "Test command",
    "",
    "Usage: test [OPTIONS] <FILE> <FILE2>",
    "",
    "Arguments:",
    "  <FILE>   Source file",
    "  <FILE2>  Destination file",
    "",
    "Options:",
    "  -j, --job <TASK>  Some task job",
    "  -h, --help        Print help",
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
