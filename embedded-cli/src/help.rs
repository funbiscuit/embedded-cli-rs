use crate::token::Tokens;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpRequest<'a> {
    /// Show list of all available commands
    All,

    /// Show help for specific command
    Command(&'a str),
}

impl<'a> HelpRequest<'a> {
    /// Tries to create new help request from input tokens
    /// If unsuccessfull, tokens are not modified and returned in Err variant
    pub fn from_tokens(mut tokens: Tokens<'a>) -> Result<Self, Tokens<'a>> {
        let mut iter = tokens.iter();

        // check if first token is help
        if tokens.iter().next() == Some("help") {
            drop(iter);
            // remove "help" token
            tokens.remove(0);
            if let Some(command) = tokens.remove(0) {
                Ok(HelpRequest::Command(command))
            } else {
                Ok(HelpRequest::All)
            }
        }
        // check if any other token is -h or --help
        else if let Some(pos) = iter.position(|token| {
            //TODO: allow to collapse -h with other options.
            // for example -vhs contains -h option
            token == "--help" || token == "-h"
        }) {
            drop(iter);
            tokens.remove(pos + 1);
            //
            let command = tokens.remove(0).unwrap();

            Ok(HelpRequest::Command(command))
        } else {
            drop(iter);
            Err(tokens)
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
    fn parsing_ok(#[case] input: &str, #[case] expected: HelpRequest<'_>) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();

        assert_eq!(HelpRequest::from_tokens(tokens), Ok(expected));
    }

    #[rstest]
    #[case("   ")]
    #[case("")]
    #[case("cmd1")]
    #[case("cmd1 help")]
    fn parsing_err(#[case] input: &str) {
        let mut input = input.as_bytes().to_vec();
        let input = core::str::from_utf8_mut(&mut input).unwrap();
        let tokens = Tokens::new(input).unwrap();

        assert!(HelpRequest::from_tokens(tokens).is_err());
    }
}
