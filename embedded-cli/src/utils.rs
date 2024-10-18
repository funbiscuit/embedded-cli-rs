use crate::utf8::Utf8Accum;

/// Returns byte index of given char index
/// If text doesn't have that many chars, returns None
/// For example, in text `abc` `b` has both char and byte index of `1`.
/// But in text `вгд` `г` has char index of 1, but byte index of `2` (`в` is 2 bytes long)
pub fn char_byte_index(text: &str, char_index: usize) -> Option<usize> {
    let mut accum = Utf8Accum::default();
    let mut byte_index = 0;
    let mut current = 0;
    for &b in text.as_bytes() {
        if char_index == current {
            return Some(byte_index);
        }
        if accum.push_byte(b).is_some() {
            current += 1;
        }
        byte_index += 1;
    }
    if char_index == current && byte_index < text.len() {
        return Some(byte_index);
    }
    None
}

pub fn char_count(text: &str) -> usize {
    let mut accum = Utf8Accum::default();
    let mut count = 0;
    for &b in text.as_bytes() {
        if accum.push_byte(b).is_some() {
            count += 1;
        }
    }
    count
}

pub fn char_pop_front(text: &str) -> Option<(char, &str)> {
    if text.is_empty() {
        None
    } else {
        let bytes = text.as_bytes();
        let first = bytes[0];

        let mut codepoint = if first < 0x80 {
            first as u32
        } else if (first & 0xE0) == 0xC0 {
            (first & 0x1F) as u32
        } else {
            (first & 0x0F) as u32
        };

        let mut bytes = &bytes[1..];
        // go over all other bytes and add merge into codepoint
        while !bytes.is_empty() && (bytes[0] & 0xC0) == 0x80 {
            codepoint <<= 6;
            codepoint |= bytes[0] as u32 & 0x3F;
            bytes = &bytes[1..];
        }

        // SAFETY: after all modifications codepoint is valid u32 char
        // and bytes contains valid utf-8 sequence
        unsafe {
            Some((
                char::from_u32_unchecked(codepoint),
                core::str::from_utf8_unchecked(bytes),
            ))
        }
    }
}

/// Returns length (in bytes) of longest common prefix
pub fn common_prefix_len(left: &str, right: &str) -> usize {
    let mut accum1 = Utf8Accum::default();

    let mut pos = 0;
    let mut byte_counter = 0;

    for (&b1, &b2) in left.as_bytes().iter().zip(right.as_bytes().iter()) {
        if b1 != b2 {
            break;
        }
        let c1 = accum1.push_byte(b1);
        byte_counter += 1;
        if c1.is_some() {
            pos = byte_counter;
        }
    }

    pos
}

/// Encodes given character as UTF-8 into the provided byte buffer,
/// and then returns the subslice of the buffer that contains the encoded character.
pub fn encode_utf8(ch: char, buf: &mut [u8]) -> &str {
    let mut code = ch as u32;

    if code < 0x80 {
        buf[0] = ch as u8;
        unsafe {
            return core::str::from_utf8_unchecked(&buf[..1]);
        }
    }

    let mut counter = if code < 0x800 {
        // 2-byte char
        1
    } else if code < 0x10000 {
        // 3-byte char
        2
    } else {
        // 4-byte char
        3
    };

    let first_b_mask = (0x780 >> counter) as u8;

    let len = counter + 1;
    while counter > 0 {
        buf[counter] = ((code as u8) & 0b0011_1111) | 0b1000_0000;
        code >>= 6;
        counter -= 1;
    }

    buf[0] = code as u8 | first_b_mask;

    unsafe {
        return core::str::from_utf8_unchecked(&buf[..len]);
    }
}

pub fn trim_start(input: &str) -> &str {
    if let Some(pos) = input.as_bytes().iter().position(|b| *b != b' ') {
        input.get(pos..).unwrap_or("")
    } else {
        ""
    }
}

/// Copies content from one slice to another (equivalent of memcpy)
///
/// # Safety
/// Length of both slices must be at least `len`
pub unsafe fn copy_nonoverlapping(src: &[u8], dst: &mut [u8], len: usize) {
    debug_assert!(src.len() >= len);
    debug_assert!(dst.len() >= len);

    // SAFETY: Caller has to check that slices have len bytes
    // and two buffers can't overlap since mutable ref is exlusive
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), len);
    }
}

/// Splits given mutable slice into two parts
///
/// # Safety
/// mid must be <= slice.len()
#[cfg(feature = "autocomplete")]
pub unsafe fn split_at_mut(buf: &mut [u8], mid: usize) -> (&mut [u8], &mut [u8]) {
    // this exists only because slice::split_at_unchecked is not stable:
    // https://github.com/rust-lang/rust/issues/76014
    let len = buf.len();
    let ptr = buf.as_mut_ptr();

    // SAFETY: Caller has to check that `mid <= self.len()`
    unsafe {
        debug_assert!(mid <= len);
        (
            core::slice::from_raw_parts_mut(ptr, mid),
            core::slice::from_raw_parts_mut(ptr.add(mid), len - mid),
        )
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use std::format;

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

    #[rstest]
    #[case("abcdef")]
    #[case("abcd абв 佐佗佟𑿁 𑿆𑿌")]
    fn char_byte_pos(#[case] text: &str) {
        // last iteration will check for None
        for pos in 0..=text.chars().count() {
            let expected = text.char_indices().map(|(pos, _)| pos).nth(pos);

            assert_eq!(utils::char_byte_index(text, pos), expected)
        }
    }

    #[rstest]
    #[case("abcdef")]
    #[case("abcd абв 佐佗佟𑿁 𑿆𑿌")]
    fn char_count(#[case] text: &str) {
        assert_eq!(utils::char_count(text), text.chars().count())
    }

    #[test]
    fn char_pop_front() {
        let text = "abcd абв 佐佗佟𑿁 𑿆𑿌";
        for (i, ch) in text.char_indices() {
            let (popped_ch, left) = utils::char_pop_front(&text[i..]).unwrap();
            assert_eq!(popped_ch, ch);
            assert_eq!(&text[i..], format!("{}{}", ch, left).as_str());
        }
        assert!(utils::char_pop_front("").is_none())
    }

    #[test]
    fn char_encode() {
        let text = "abcd абв 佐佗佟𑿁 𑿆𑿌";
        for ch in text.chars() {
            let mut buf1 = [0; 4];
            let mut buf2 = [0; 4];
            assert_eq!(ch.encode_utf8(&mut buf1), utils::encode_utf8(ch, &mut buf2));
        }
        assert!(utils::char_pop_front("").is_none())
    }

    #[rstest]
    #[case("abcdef", "abcdef")]
    #[case("abcdef", "abc")]
    #[case("abcdef", "abc ghf")]
    #[case("abcdef", "")]
    #[case("", "")]
    #[case("абв 佐佗佟𑿁", "абв 佐佗佟𑿁")]
    #[case("абв 佐佗佟𑿁𑿆𑿌", "абв 佐佗佟𑿁")]
    #[case("абв 佐佗佟𑿁 𑿆𑿌", "абв 佐佗𑿁佟")]
    fn common_prefix(#[case] left: &str, #[case] right: &str) {
        let expected = left
            .char_indices()
            .zip(right.char_indices())
            .find(|((_, a1), (_, a2))| a1 != a2)
            .map(|((pos, _), _)| pos)
            .unwrap_or(right.len().min(left.len()));

        assert_eq!(utils::common_prefix_len(left, right), expected);
        assert_eq!(utils::common_prefix_len(right, left), expected);
    }
}
