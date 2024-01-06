pub fn trim_start(input: &str) -> &str {
    if let Some(pos) = input.as_bytes().iter().position(|b| *b != b' ') {
        input.get(pos..).unwrap_or("")
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::utils;

    #[rstest]
    #[case::no_spaces("abc", "abc")]
    #[case::leading_spaces("  abc", "abc")]
    #[case::trailing_spaces("abc  ", "abc  ")]
    #[case::both_spaces("  abc   ", "abc   ")]
    #[case::space_inside("  abc def  ", "abc def  ")]
    #[case::multiple_spaces_inside("  abc   def  ", "abc   def  ")]
    #[case::utf8("  abc dабвг佐  佗佟𑿁   𑿆𑿌  ", "abc dабвг佐  佗佟𑿁   𑿆𑿌  ")]
    fn trim_start(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(utils::trim_start(input), expected);
    }
}
