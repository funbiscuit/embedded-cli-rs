pub const BACKSPACE: u8 = 0x08;
pub const TABULATION: u8 = 0x09;
pub const LINE_FEED: u8 = 0x0A;
pub const CARRIAGE_RETURN: u8 = 0x0D;
pub const ESCAPE: u8 = 0x1B;

pub const CRLF: &str = "\r\n";

// escape sequence reference: https://ecma-international.org/publications-and-standards/standards/ecma-48
pub const CURSOR_FORWARD: &[u8] = b"\x1B[C";
pub const CURSOR_BACKWARD: &[u8] = b"\x1B[D";
pub const INSERT_CHAR: &[u8] = b"\x1B[@";
pub const DELETE_CHAR: &[u8] = b"\x1B[P";
