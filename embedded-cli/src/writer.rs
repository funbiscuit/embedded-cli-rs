use core::{convert::Infallible, fmt::Debug};

use embedded_io::{Error, ErrorType, Write};
use ufmt::uWrite;

use crate::codes;

pub struct Writer<'a, W: Write<Error = E>, E: Error> {
    last_bytes: [u8; 2],
    dirty: bool,
    writer: &'a mut W,
}

impl<'a, W: Write<Error = E>, E: Error> Debug for Writer<'a, W, E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Writer")
            .field("last_bytes", &self.last_bytes)
            .field("dirty", &self.dirty)
            .finish()
    }
}

impl<'a, W: Write<Error = E>, E: Error> Writer<'a, W, E> {
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            last_bytes: [0; 2],
            dirty: false,
            writer,
        }
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
            && (self.last_bytes[0] != codes::CARRIAGE_RETURN
                || self.last_bytes[1] != codes::LINE_FEED)
    }

    pub fn write_str(&mut self, mut text: &str) -> Result<(), E> {
        while !text.is_empty() {
            if let Some(pos) = text.as_bytes().iter().position(|&b| b == codes::LINE_FEED) {
                // SAFETY: pos is inside text slice
                let line = unsafe { text.get_unchecked(..pos) };

                self.writer.write_str(line)?;
                self.writer.write_str(codes::CRLF)?;
                // SAFETY: pos is index of existing element so pos + 1 in worst case will be
                // outside of slice by 1, which is safe (will give empty slice as result)
                text = unsafe { text.get_unchecked(pos + 1..) };
                self.dirty = false;
                self.last_bytes = [0; 2];
            } else {
                self.writer.write_str(text)?;
                self.dirty = true;

                if text.len() > 1 {
                    self.last_bytes[0] = text.as_bytes()[text.len() - 2];
                    self.last_bytes[1] = text.as_bytes()[text.len() - 1];
                } else {
                    self.last_bytes[0] = self.last_bytes[1];
                    self.last_bytes[1] = text.as_bytes()[text.len() - 1];
                }
                break;
            }
        }
        Ok(())
    }

    pub fn writeln_str(&mut self, text: &str) -> Result<(), E> {
        self.writer.write_str(text)?;
        self.writer.write_str(codes::CRLF)?;
        self.dirty = false;
        Ok(())
    }

    pub fn write_list_element(
        &mut self,
        name: &str,
        description: &str,
        longest_name: usize,
    ) -> Result<(), E> {
        self.write_str("  ")?;
        self.write_str(name)?;
        if name.len() < longest_name {
            for _ in 0..longest_name - name.len() {
                self.write_str(" ")?;
            }
        }
        self.write_str("  ")?;
        self.writeln_str(description)?;

        Ok(())
    }

    pub fn write_title(&mut self, title: &str) -> Result<(), E> {
        //TODO: add formatting
        self.write_str(title)?;
        Ok(())
    }
}

impl<'a, W: Write<Error = E>, E: Error> uWrite for Writer<'a, W, E> {
    type Error = E;

    fn write_str(&mut self, s: &str) -> Result<(), E> {
        self.write_str(s)
    }
}

impl<'a, W: Write<Error = E>, E: Error> core::fmt::Write for Writer<'a, W, E> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s).map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}

pub(crate) trait WriteExt: ErrorType {
    /// Write and flush all given bytes
    fn flush_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn flush_str(&mut self, text: &str) -> Result<(), Self::Error>;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error>;
}

impl<W: Write> WriteExt for W {
    fn flush_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_bytes(bytes)?;
        self.flush()
    }

    fn flush_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.flush_bytes(text.as_bytes())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_all(bytes)
    }

    fn write_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.write_bytes(text.as_bytes())
    }
}

#[derive(Debug)]
pub struct EmptyWriter;

impl ErrorType for EmptyWriter {
    type Error = Infallible;
}

impl Write for EmptyWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::writer::{EmptyWriter, Writer};

    #[test]
    fn detect_dirty() {
        let mut writer = EmptyWriter;
        let mut writer = Writer::new(&mut writer);

        assert!(!writer.is_dirty());

        writer.write_str("abc").unwrap();
        assert!(writer.is_dirty());

        writer.write_str("\r").unwrap();
        assert!(writer.is_dirty());

        writer.write_str("\n").unwrap();
        assert!(!writer.is_dirty());

        writer.write_str("abc\r\n").unwrap();
        assert!(!writer.is_dirty());
    }
}
