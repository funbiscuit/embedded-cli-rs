use crate::{
    arguments::{Arg, ArgList},
    token::Tokens,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpRequest<'a> {
    /// Show list of all available commands
    All,

    /// Show help for specific command
    Command(&'a str),
}

impl<'a> HelpRequest<'a> {
    /// Tries to create new help request from input tokens
    pub fn from_tokens(tokens: &Tokens<'a>) -> Option<Self> {
        let mut iter = tokens.iter();
        let command = iter.next()?;

        // check if first token is help
        if command == "help" {
            if let Some(command) = iter.next() {
                Some(HelpRequest::Command(command))
            } else {
                Some(HelpRequest::All)
            }
        }
        // check if any other option is -h or --help
        else if ArgList::new(iter.into_tokens())
            .args()
            .flatten()
            .any(|arg| arg == Arg::LongOption("help") || arg == Arg::ShortOption('h'))
        {
            Some(HelpRequest::Command(command))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::token::Tokens;

    use super::HelpRequest;

    #[rstest]
    #[case("help", HelpRequest::All)]
    #[case("help cmd1", HelpRequest::Command("cmd1"))]
    #[case("cmd2 --help", HelpRequest::Command("cmd2"))]
    #[case("cmd3 -v --opt --help --some", HelpRequest::Command("cmd3"))]
    #[case("cmd3 -vh --opt --some", HelpRequest::Command("cmd3"))]
    #[case("cmd3 -hv --opt --some", HelpRequest::Command("cmd3"))]
    fn parsing_ok(#[case] input: &str, #[case] expected: HelpRequest<'_>) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();

        assert_eq!(HelpRequest::from_tokens(&tokens), Some(expected));
    }

    #[rstest]
    #[case("   ")]
    #[case("")]
    #[case("cmd1")]
    #[case("cmd1 help")]
    #[case("--help")]
    fn parsing_err(#[case] input: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();
        let res = HelpRequest::from_tokens(&tokens);
        std::dbg!(&res);
        assert!(res.is_none());
    }
}
