//! Little-endian primitives of the GP3 to GP5 formats: a bounds-checked reader that
//! never panics and never allocates more than the input can hold, a writer, and the
//! text encoding (ADR-036 choice 7.3).

/// Where reading stopped and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadError {
    pub at: usize,
    pub what: String,
}

pub(crate) type Res<T> = Result<T, ReadError>;

/// Longest string the reader accepts (ADR-036 section 3).
pub(crate) const MAX_STRING: usize = 1 << 20;

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn fail<T>(&self, what: impl Into<String>) -> Res<T> {
        Err(ReadError {
            at: self.pos,
            what: what.into(),
        })
    }

    pub fn bytes(&mut self, n: usize) -> Res<&'a [u8]> {
        if n > self.remaining() {
            return self.fail(format!(
                "unexpected end of file: {n} bytes needed, {} left",
                self.remaining()
            ));
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn skip(&mut self, n: usize) -> Res<()> {
        self.bytes(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Res<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn i8(&mut self) -> Res<i8> {
        Ok(self.u8()? as i8)
    }

    pub fn bool(&mut self) -> Res<bool> {
        Ok(self.u8()? != 0)
    }

    pub fn i16(&mut self) -> Res<i16> {
        let b = self.bytes(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i32(&mut self) -> Res<i32> {
        let b = self.bytes(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// An Int count that must lie in `0..=max`, so nothing is allocated without bound.
    pub fn count(&mut self, max: usize, what: &str) -> Res<usize> {
        let at = self.pos;
        let n = self.i32()?;
        if n < 0 || n as usize > max {
            return Err(ReadError {
                at,
                what: format!("{what}: {n} is outside 0..={max}"),
            });
        }
        Ok(n as usize)
    }

    /// Byte length, then a buffer of `size` bytes of which the first `length` are text.
    pub fn byte_string_fixed(&mut self, size: usize) -> Res<String> {
        let len = self.u8()? as usize;
        let buf = self.bytes(size)?;
        if len > size {
            return self.fail(format!("text length {len} is over its buffer of {size}"));
        }
        Ok(decode(&buf[..len]))
    }

    /// Int length, then the text.
    pub fn int_string(&mut self) -> Res<String> {
        let len = self.count(MAX_STRING, "text length")?;
        Ok(decode(self.bytes(len)?))
    }

    /// Int (length + 1), Byte length, then the text.
    pub fn int_byte_string(&mut self) -> Res<String> {
        let at = self.pos;
        let outer = self.count(MAX_STRING, "text length")?;
        let len = self.u8()? as usize;
        if outer != len + 1 && !(outer == 0 && len == 0) {
            return Err(ReadError {
                at,
                what: format!("text lengths disagree: {outer} and {len}"),
            });
        }
        Ok(decode(self.bytes(len)?))
    }
}

#[derive(Debug, Default)]
pub(crate) struct Writer {
    pub out: Vec<u8>,
}

impl Writer {
    pub fn u8(&mut self, v: u8) {
        self.out.push(v);
    }

    pub fn i8(&mut self, v: i8) {
        self.out.push(v as u8);
    }

    pub fn bool(&mut self, v: bool) {
        self.out.push(v as u8);
    }

    pub fn i16(&mut self, v: i16) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    pub fn i32(&mut self, v: i32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bytes(&mut self, v: &[u8]) {
        self.out.extend_from_slice(v);
    }

    /// Byte length and a fixed buffer. Returns false if the text was cut.
    pub fn byte_string_fixed(&mut self, s: &str, size: usize) -> bool {
        let t = cut(s, size);
        self.u8(t.len() as u8);
        self.bytes(t);
        self.bytes(&vec![0; size - t.len()]);
        t.len() == s.len()
    }

    pub fn int_string(&mut self, s: &str) {
        self.i32(s.len() as i32);
        self.bytes(s.as_bytes());
    }

    /// Int (length + 1), Byte length, text of at most 255 bytes. Returns false if cut.
    pub fn int_byte_string(&mut self, s: &str) -> bool {
        let t = cut(s, 255);
        self.i32(t.len() as i32 + 1);
        self.u8(t.len() as u8);
        self.bytes(t);
        t.len() == s.len()
    }
}

/// The longest prefix of `s` of at most `max` bytes that ends on a character boundary.
fn cut(s: &str, max: usize) -> &[u8] {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s.as_bytes()[..end]
}

/// UTF-8 when the bytes are valid UTF-8 (TuxGuitar's default), else Windows-1252.
pub(crate) fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| windows_1252(b)).collect(),
    }
}

/// Windows-1252: Latin-1 except 0x80 to 0x9F. The five undefined bytes map to the
/// C1 control of the same value, as Windows does.
fn windows_1252(b: u8) -> char {
    const HIGH: [u16; 32] = [
        0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022,
        0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) {
        char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('\u{FFFD}')
    } else {
        b as char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_utf8_when_valid_else_windows_1252() {
        assert_eq!(decode("Bài hát".as_bytes()), "Bài hát");
        assert_eq!(decode(&[0x80, b'5', 0xE9]), "€5é");
    }

    #[test]
    fn cut_keeps_whole_characters() {
        let mut w = Writer::default();
        assert!(!w.byte_string_fixed("aé", 2));
        assert_eq!(w.out, [1, b'a', 0]);
    }

    #[test]
    fn counts_are_bounded() {
        let mut r = Reader::new(&[0xff, 0xff, 0xff, 0x7f]);
        assert!(r.count(10, "x").is_err());
        let mut r = Reader::new(&[1, 2]);
        assert!(r.i32().is_err());
    }
}
