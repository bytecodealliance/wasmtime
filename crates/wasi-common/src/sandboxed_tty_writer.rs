use std::io::{Result, Write};

/// An adapter around a `Write` stream that guarantees that its output
/// is valid UTF-8 and contains no control characters. It does this by
/// replacing characters with inert control pictures and replacement
/// characters.
pub(crate) struct SandboxedTTYWriter<'writer, Writer>
where
    Writer: Write,
{
    inner: &'writer mut Writer,
    scratch: [u8; 4],
}

impl<'writer, Writer> SandboxedTTYWriter<'writer, Writer>
where
    Writer: Write,
{
    /// Construct a new `SandboxedTTYWriter` with the given inner `Writer`.
    pub(crate) fn new(inner: &'writer mut Writer) -> Self {
        Self {
            inner,
            scratch: [0; 4],
        }
    }

    /// Write a single character to the output.
    pub(crate) fn write_char(&mut self, c: char) -> Result<()> {
        self.inner.write(
            match c {
                '\u{0000}' => '␀',
                '\u{0001}' => '␁',
                '\u{0002}' => '␂',
                '\u{0003}' => '␃',
                '\u{0004}' => '␄',
                '\u{0005}' => '␅',
                '\u{0006}' => '␆',
                '\u{0007}' => '␇',
                '\u{0008}' => '␈',
                '\u{0009}' => '\t',
                '\u{000A}' => '\n',
                '\u{000B}' => '␋',
                '\u{000C}' => '␌',
                '\u{000D}' => '\r',
                '\u{000E}' => '␎',
                '\u{000F}' => '␏',
                '\u{0010}' => '␐',
                '\u{0011}' => '␑',
                '\u{0012}' => '␒',
                '\u{0013}' => '␓',
                '\u{0014}' => '␔',
                '\u{0015}' => '␕',
                '\u{0016}' => '␖',
                '\u{0017}' => '␗',
                '\u{0018}' => '␘',
                '\u{0019}' => '␙',
                '\u{001A}' => '␚',
                '\u{001B}' => '␛',
                '\u{001C}' => '␜',
                '\u{001D}' => '␝',
                '\u{001E}' => '␞',
                '\u{001F}' => '␟',
                '\u{007F}' => '␡',
                x if x.is_control() => '�',
                x => x,
            }
            .encode_utf8(&mut self.scratch)
            .as_bytes(),
        )?;

        Ok(())
    }

    /// Write a string to the output.
    pub(crate) fn write_str(&mut self, s: &str) -> Result<usize> {
        let mut result = 0;

        for c in s.chars() {
            self.write_char(c)?;
            // Note that use the encoding length of the given char, rather than
            // how many bytes we actually wrote, because our users don't know about
            // what's really being written.
            result += c.len_utf8();
        }

        Ok(result)
    }
}

impl<'writer, Writer> Write for SandboxedTTYWriter<'writer, Writer>
where
    Writer: Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut input = buf;
        let mut result = 0;

        // Decode the string without heap-allocating it. See the example here
        // for more details:
        // https://doc.rust-lang.org/std/str/struct.Utf8Error.html#examples
        loop {
            match ::std::str::from_utf8(input) {
                Ok(valid) => {
                    result += self.write_str(valid)?;
                    break;
                }
                Err(error) => {
                    let (valid, after_valid) = input.split_at(error.valid_up_to());
                    result += self.write_str(unsafe { std::str::from_utf8_unchecked(valid) })?;
                    self.write_char('�')?;

                    if let Some(invalid_sequence_length) = error.error_len() {
                        result += invalid_sequence_length;
                        input = &after_valid[invalid_sequence_length..];
                    } else {
                        result += after_valid.len();
                        break;
                    }
                }
            }
        }

        return Ok(result);
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::SandboxedTTYWriter;
    use std::io::{Result, Write};

    #[test]
    fn basic() -> Result<()> {
        let mut safe = SandboxedTTYWriter::new(Vec::new());
        safe.write_str("a\0b\u{0080}")?;
        safe.write_char('\u{0007}')?;
        safe.write(&[0x80])?;
        safe.write(&[0xed, 0xa0, 0x80, 0xff, 0xfe])?;
        let output = safe.into_inner_writer();
        assert_eq!(
            output,
            "a\u{2400}b\u{FFFD}\u{2407}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}".as_bytes()
        );
        Ok(())
    }

    #[test]
    fn how_many_replacements() -> Result<()> {
        // See https://hsivonen.fi/broken-utf-8/ for background.

        let mut safe = SandboxedTTYWriter::new(Vec::new());
        safe.write(&[0x80, 0x80, 0x80, 0x80])?;
        let output = safe.into_inner_writer();
        assert_eq!(output, "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}".as_bytes());

        let mut safe = SandboxedTTYWriter::new(Vec::new());
        safe.write(&[0xF0, 0x80, 0x80, 0x41])?;
        let output = safe.into_inner_writer();
        assert_eq!(output, "\u{FFFD}\u{FFFD}\u{FFFD}A".as_bytes());

        let mut safe = SandboxedTTYWriter::new(Vec::new());
        safe.write(&[0xF0, 0x80, 0x80])?;
        let output = safe.into_inner_writer();
        assert_eq!(output, "\u{FFFD}\u{FFFD}\u{FFFD}".as_bytes());

        let mut safe = SandboxedTTYWriter::new(Vec::new());
        safe.write(&[0xF4, 0x80, 0x80, 0xC0])?;
        let output = safe.into_inner_writer();
        assert_eq!(output, "\u{FFFD}\u{FFFD}".as_bytes());

        Ok(())
    }
}
