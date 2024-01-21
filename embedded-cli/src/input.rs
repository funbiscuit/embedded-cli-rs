use bitflags::bitflags;

use crate::{codes, utf8::Utf8Accum};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlInput {
    Backspace,
    Down,
    Enter,
    Back,
    Forward,
    Tab,
    Up,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Input<'a> {
    Control(ControlInput),

    /// Input is a single utf8 char.
    /// Slice is used to skip conversion from char to byte slice
    Char(&'a str),
}

bitflags! {
    #[derive(Debug)]
    struct Flags: u8 {
        const CSI_STARTED = 1;
    }
}

#[derive(Debug)]
pub struct InputGenerator {
    flags: Flags,
    last_byte: u8,
    utf8: Utf8Accum,
}

impl InputGenerator {
    pub fn new() -> Self {
        // last byte matters only when its Esc, \r or \n, so can set it to just 0
        Self {
            flags: Flags::empty(),
            last_byte: 0,
            utf8: Utf8Accum::default(),
        }
    }

    pub fn accept(&mut self, byte: u8) -> Option<Input<'_>> {
        let last_byte = self.last_byte;
        self.last_byte = byte;
        if self.flags.contains(Flags::CSI_STARTED) {
            self.process_csi(byte).map(Input::Control)
        } else if last_byte == codes::ESCAPE && byte == b'[' {
            self.flags.set(Flags::CSI_STARTED, true);
            None
        } else {
            self.process_single(byte, last_byte)
        }
    }

    fn process_csi(&mut self, byte: u8) -> Option<ControlInput> {
        // skip all parameter bytes and process only last byte in CSI sequence
        if (0x40..=0x7E).contains(&byte) {
            self.flags.set(Flags::CSI_STARTED, false);
            let control = match byte {
                b'A' => ControlInput::Up,
                b'B' => ControlInput::Down,
                b'C' => ControlInput::Forward,
                b'D' => ControlInput::Back,
                _ => return None,
            };
            Some(control)
        } else {
            None
        }
    }

    fn process_single(&mut self, byte: u8, last_byte: u8) -> Option<Input<'_>> {
        let control = match byte {
            codes::BACKSPACE => ControlInput::Backspace,

            // ignore \r if \n already received (and converted to Enter)
            codes::CARRIAGE_RETURN if last_byte != codes::LINE_FEED => ControlInput::Enter,

            // ignore \n if \r already received (and converted to Enter)
            codes::LINE_FEED if last_byte != codes::CARRIAGE_RETURN => ControlInput::Enter,

            codes::TABULATION => ControlInput::Tab,

            // process only non control ascii chars (and utf8)
            byte if byte >= 0x20 => return self.utf8.push_byte(byte).map(Input::Char),

            _ => return None,
        };
        Some(Input::Control(control))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{ControlInput, Input, InputGenerator};

    #[rstest]
    #[case(b"\x1B[A", ControlInput::Up)]
    #[case(b"\x1B[B", ControlInput::Down)]
    #[case(b"\x1B[24B", ControlInput::Down)]
    #[case(b"\x1B[C", ControlInput::Forward)]
    #[case(b"\x1B[D", ControlInput::Back)]
    fn process_csi_control(#[case] bytes: &[u8], #[case] expected: ControlInput) {
        let mut accum = InputGenerator::new();

        for &b in &bytes[..bytes.len() - 1] {
            assert_eq!(accum.accept(b), None);
        }

        assert_eq!(
            accum.accept(*bytes.last().unwrap()),
            Some(Input::Control(expected))
        )
    }

    #[rstest]
    #[case(0x08, ControlInput::Backspace)]
    #[case(b'\t', ControlInput::Tab)]
    #[case(b'\r', ControlInput::Enter)]
    #[case(b'\n', ControlInput::Enter)]
    fn process_c0_control(#[case] byte: u8, #[case] expected: ControlInput) {
        assert_eq!(
            InputGenerator::new().accept(byte),
            Some(Input::Control(expected))
        )
    }

    #[test]
    fn process_crlf() {
        let mut accum = InputGenerator::new();
        accum.accept(b'\r');

        assert_eq!(accum.accept(b'\n'), None);
        assert_eq!(accum.accept(b'a'), Some(Input::Char("a")));
    }

    #[test]
    fn process_lfcr() {
        let mut accum = InputGenerator::new();
        accum.accept(b'\n');

        assert_eq!(accum.accept(b'\r'), None);
        assert_eq!(accum.accept(b'a'), Some(Input::Char("a")));
    }

    #[test]
    fn process_input() {
        let mut accum = InputGenerator::new();

        assert_eq!(accum.accept(b'a'), Some(Input::Char("a")));
        assert_eq!(accum.accept(b'b'), Some(Input::Char("b")));
        assert_eq!(accum.accept("б".as_bytes()[0]), None);
        assert_eq!(accum.accept("б".as_bytes()[1]), Some(Input::Char("б")));
        assert_eq!(
            accum.accept(b'\n'),
            Some(Input::Control(ControlInput::Enter))
        );
        assert_eq!(accum.accept(b'a'), Some(Input::Char("a")));
        assert_eq!(accum.accept(b'b'), Some(Input::Char("b")));
        assert_eq!(accum.accept(b'\t'), Some(Input::Control(ControlInput::Tab)));
        assert_eq!(accum.accept(0x1B), None);
        assert_eq!(accum.accept(b'['), None);
        assert_eq!(accum.accept(b'A'), Some(Input::Control(ControlInput::Up)));
        assert_eq!(accum.accept(0x1B), None);
        assert_eq!(accum.accept(b'['), None);
        assert_eq!(accum.accept(b'B'), Some(Input::Control(ControlInput::Down)));
        assert_eq!(accum.accept(b'a'), Some(Input::Char("a")));
        assert_eq!(accum.accept(b'b'), Some(Input::Char("b")));
        assert_eq!(accum.accept(0x1B), None);
        assert_eq!(accum.accept(b'['), None);
        assert_eq!(accum.accept(b'B'), Some(Input::Control(ControlInput::Down)));
    }
}
