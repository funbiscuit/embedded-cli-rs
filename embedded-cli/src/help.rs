use crate::{arguments::Arg, command::RawCommand};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpRequest<'a> {
    /// Show list of all available commands
    All,

    /// Show help for specific command with arguments
    /// One of command arguments might be -h or --help
    Command(RawCommand<'a>),
}

impl<'a> HelpRequest<'a> {
    /// Tries to create new help request from raw command
    pub fn from_command(command: &RawCommand<'a>) -> Option<Self> {
        let mut args = command.args().args();
        if command.name() == "help" {
            match args.next() {
                Some(Arg::Value(name)) => {
                    let command = RawCommand::new(name, args.into_args());
                    Some(HelpRequest::Command(command))
                }
                None => Some(HelpRequest::All),
                _ => None,
            }
        }
        // check if any other option is -h or --help
        else if args.any(|arg| arg == Arg::LongOption("help") || arg == Arg::ShortOption('h')) {
            Some(HelpRequest::Command(command.clone()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{arguments::ArgList, command::RawCommand, token::Tokens};

    use super::HelpRequest;

    fn help_command(name: &'static str, args: &'static str) -> HelpRequest<'static> {
        HelpRequest::Command(RawCommand::new(
            name,
            ArgList::new(Tokens::from_raw(args, args.is_empty())),
        ))
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
        let command = RawCommand::from_tokens(&tokens).unwrap();

        assert_eq!(HelpRequest::from_command(&command), Some(expected));
    }

    #[rstest]
    #[case("cmd1")]
    #[case("cmd1 help")]
    #[case("--help")]
    fn parsing_err(#[case] input: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input);
        let command = RawCommand::from_tokens(&tokens).unwrap();
        let res = HelpRequest::from_command(&command);

        assert!(res.is_none());
    }
}
