#[cfg(feature = "help")]
use {
    crate::arguments::{Arg, Args},
    crate::writer::Writer,
    embedded_io::Write,
};

#[derive(Debug)]
pub enum HelpError<E: embedded_io::Error> {
    WriteError(E),
    UnknownCommand,
}

impl<E: embedded_io::Error> From<E> for HelpError<E> {
    fn from(value: E) -> Self {
        Self::WriteError(value)
    }
}

// trait is kept available so it's possible to use same where clause
pub trait Help {
    #[cfg(feature = "help")]
    /// How many commands are known
    fn command_count() -> usize;

    #[cfg(feature = "help")]
    /// Print all commands and short description of each
    fn list_commands<W: Write<Error = E>, E: embedded_io::Error>(
        writer: &mut Writer<'_, W, E>,
    ) -> Result<(), E>;

    #[cfg(feature = "help")]
    /// Print help for given command. Arguments might contain -h or --help options
    /// Use given writer to print help text
    /// If help request cannot be processed by this object,
    /// Err(HelpError::UnknownCommand) must be returned
    fn command_help<
        W: Write<Error = E>,
        E: embedded_io::Error,
        F: FnMut(&mut Writer<'_, W, E>) -> Result<(), E>,
    >(
        parent: &mut F,
        name: &str,
        args: Args<'_>,
        writer: &mut Writer<'_, W, E>,
    ) -> Result<(), HelpError<E>>;
}

#[cfg(feature = "help")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HelpRequest<'a> {
    /// Show list of all available commands
    All,

    /// Show help for specific command with arguments
    /// One of command arguments might be -h or --help
    Command { name: &'a str, args: Args<'a> },
}

#[cfg(feature = "help")]
impl<'a> HelpRequest<'a> {
    /// Tries to create new help request from command name and arguments
    pub fn from_command(name: &'a str, args: &Args<'a>) -> Option<Self> {
        let mut args_iter = args.iter();
        if name == "help" {
            match args_iter.next() {
                Some(Arg::Value(name)) => Some(HelpRequest::Command {
                    name,
                    args: args_iter.into_args(),
                }),
                _ => Some(HelpRequest::All),
            }
        }
        // check if any other option is -h or --help
        else if args_iter
            .any(|arg| arg == Arg::LongOption("help") || arg == Arg::ShortOption('h'))
        {
            Some(HelpRequest::Command {
                name,
                args: args.clone(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{arguments::Args, token::Tokens};

    use super::HelpRequest;

    fn help_command(name: &'static str, args: &'static str) -> HelpRequest<'static> {
        HelpRequest::Command {
            name,
            args: Args::new(Tokens::from_raw(args, args.is_empty())),
        }
    }

    #[rstest]
    #[case("help", HelpRequest::All)]
    #[case("help cmd1", help_command("cmd1", ""))]
    #[case("cmd2 --help", help_command("cmd2", "--help"))]
    #[case(
        "cmd3 -v --opt --help --some",
        help_command("cmd3", "-v\0--opt\0--help\0--some")
    )]
    #[case("cmd3 -vh --opt --some", help_command("cmd3", "-vh\0--opt\0--some"))]
    #[case("cmd3 -hv --opt --some", help_command("cmd3", "-hv\0--opt\0--some"))]
    fn parsing_ok(#[case] input: &str, #[case] expected: HelpRequest<'_>) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let (name, tokens) = tokens.split_first().unwrap();
        let args = Args::new(tokens);

        assert_eq!(HelpRequest::from_command(name, &args), Some(expected));
    }

    #[rstest]
    #[case("cmd1")]
    #[case("cmd1 help")]
    #[case("--help")]
    fn parsing_err(#[case] input: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let (name, tokens) = tokens.split_first().unwrap();
        let args = Args::new(tokens);
        let res = HelpRequest::from_command(name, &args);

        assert!(res.is_none());
    }
}
