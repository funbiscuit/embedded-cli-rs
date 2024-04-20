#[derive(Debug, Default)]
pub struct Utf8Accum {
    /// Buffer for utf8 octets aggregation until full utf-8 char is received
    buffer: [u8; 4],

    /// How many more utf8 octets are expected
    expected: u8,

    /// How many utf8 octets are in the buffer
    partial: u8,
}

impl Utf8Accum {
    pub fn push_byte(&mut self, byte: u8) -> Option<&str> {
        // Plain and stupid utf-8 validation
        // Bytes are supposed to be human input so it's okay to be not blazing fast

        if byte >= 0xF8 {
            return None;
        } else if byte >= 0xF0 {
            // this is first octet of 4-byte value
            self.buffer[0] = byte;
            self.partial = 1;
            self.expected = 3;
        } else if byte >= 0xE0 {
            // this is first octet of 3-byte value
            self.buffer[0] = byte;
            self.partial = 1;
            self.expected = 2;
        } else if byte >= 0xC0 {
            // this is first octet of 2-byte value
            self.buffer[0] = byte;
            self.partial = 1;
            self.expected = 1;
        } else if byte >= 0x80 {
            if self.expected > 0 {
                // this is one of other octets of multi-byte value
                self.buffer[self.partial as usize] = byte;
                self.partial += 1;
                self.expected -= 1;
                if self.expected == 0 {
                    let len = self.partial as usize;
                    // SAFETY: we checked previously that buffer contains valid utf8
                    unsafe {
                        return Some(core::str::from_utf8_unchecked(&self.buffer[..len]));
                    }
                }
            }
        } else {
            self.expected = 0;
            self.buffer[0] = byte;
            // SAFETY: ascii chars are all valid utf-8 chars
            unsafe {
                return Some(core::str::from_utf8_unchecked(&self.buffer[..1]));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::string::String;

    use crate::utf8::Utf8Accum;

    #[test]
    fn utf8_support() {
        let mut accum = Utf8Accum::default();

        let expected_str = "abcdабвг佐佗佟𑿁𑿆𑿌";

        let mut text = String::new();

        for &b in expected_str.as_bytes() {
            if let Some(t) = accum.push_byte(b) {
                text.push_str(t);
            }
        }

        assert_eq!(text, expected_str);
    }
}
