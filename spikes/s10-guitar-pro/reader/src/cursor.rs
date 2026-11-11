//! Little-endian primitives of the GP3 to GP5 formats, with bounds checks and an
//! optional trace of named sections and their offsets.

#[derive(Debug)]
pub struct Error {
    pub at: usize,
    pub what: String,
}

pub type Res<T> = Result<T, Error>;

pub struct Cursor<'a> {
    data: &'a [u8],
    pub pos: usize,
    pub trace: bool,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Cursor {
            data,
            pos: 0,
            trace: false,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn fail<T>(&self, what: impl Into<String>) -> Res<T> {
        Err(Error {
            at: self.pos,
            what: what.into(),
        })
    }

    pub fn mark(&self, name: &str) {
        if self.trace {
            println!("  @{:>6} {name}", self.pos);
        }
    }

    pub fn bytes(&mut self, n: usize) -> Res<&'a [u8]> {
        if n > self.remaining() {
            return self.fail(format!("need {n} bytes, {} left", self.remaining()));
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

    pub fn f64(&mut self) -> Res<f64> {
        let b = self.bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(f64::from_le_bytes(a))
    }

    /// A count read as an Int that must lie in `0..=max` (no unbounded allocation).
    pub fn count(&mut self, max: usize, what: &str) -> Res<usize> {
        let at = self.pos;
        let n = self.i32()?;
        if n < 0 || n as usize > max {
            return Err(Error {
                at,
                what: format!("{what}: count {n} not in 0..={max}"),
            });
        }
        Ok(n as usize)
    }

    /// Text bytes, decoded by the caller's choice (UTF-8 if valid, else Latin-1).
    fn text(bytes: &[u8]) -> String {
        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect(),
        }
    }

    /// ByteSizeString in a fixed buffer of `size` bytes (length byte, then `size` bytes).
    pub fn byte_string_fixed(&mut self, size: usize) -> Res<String> {
        let len = self.u8()? as usize;
        let buf = self.bytes(size)?;
        if len > size {
            return self.fail(format!("string length {len} over its buffer of {size}"));
        }
        Ok(Self::text(&buf[..len]))
    }

    /// ByteSizeString without padding.
    pub fn byte_string(&mut self) -> Res<String> {
        let len = self.u8()? as usize;
        Ok(Self::text(self.bytes(len)?))
    }

    /// IntSizeString: an Int length, then the bytes.
    pub fn int_string(&mut self) -> Res<String> {
        let len = self.count(1 << 20, "string")?;
        Ok(Self::text(self.bytes(len)?))
    }

    /// IntByteSizeString: an Int (length + 1), a Byte length, then the bytes.
    pub fn int_byte_string(&mut self) -> Res<String> {
        let at = self.pos;
        let outer = self.count(1 << 20, "string")?;
        let len = self.u8()? as usize;
        if outer != len + 1 && !(outer == 0 && len == 0) {
            return Err(Error {
                at,
                what: format!("IntByteSizeString sizes disagree: {outer} and {len}"),
            });
        }
        Ok(Self::text(self.bytes(len)?))
    }
}
