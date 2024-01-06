/// Function to rotate `buf` by `mid` elements
///
/// Not using `core::slice::rotate_left` since it
/// contains assertion that generates panic
/// and there is no unsafe version
pub fn rotate_left(buf: &mut [u8], mid: usize) {
    let n = buf.len();
    if n == 0 || mid == 0 {
        return;
    }

    // To handle mid >= n
    let d = mid % n;
    for i in 0..gcd(d, n) {
        // move i-th values of blocks
        let temp = buf[i];
        let mut j = i;

        loop {
            let mut k = j + d;
            if k >= n {
                k = k - n;
            }

            if k == i {
                break;
            }

            buf[j] = buf[k];
            j = k;
        }
        buf[j] = temp;
    }
}

pub fn trim_start(input: &str) -> &str {
    if let Some(pos) = input.as_bytes().iter().position(|b| *b != b' ') {
        input.get(pos..).unwrap_or("")
    } else {
        ""
    }
}

/// Splits given mutable slice into two parts
///
/// # Safety
/// mid must be <= slice.len()
pub unsafe fn split_at_mut<'a>(buf: &'a mut [u8], mid: usize) -> (&'a mut [u8], &'a mut [u8]) {
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

fn gcd(mut a: usize, mut b: usize) -> usize {
    while a > 0 && b > 0 {
        if a > b {
            a = a % b;
        } else {
            b = b % a;
        }
    }
    if a == 0 {
        b
    } else {
        a
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

    #[rstest]
    #[case(2, 3, 1)]
    #[case(20, 28, 4)]
    #[case(60, 36, 12)]
    #[case(98, 56, 14)]
    fn gcd(#[case] a: usize, #[case] b: usize, #[case] result: usize) {
        assert_eq!(utils::gcd(a, b), result);
    }

    #[rstest]
    #[case(&[1, 2, 3, 4, 5, 6, 7], 2, &[3, 4, 5, 6, 7, 1, 2])]
    fn left_rotate(#[case] input: &[u8], #[case] n: usize, #[case] result: &[u8]) {
        let mut input = input.to_vec();
        utils::rotate_left(&mut input, n);
        assert_eq!(&input, result);
    }
}
