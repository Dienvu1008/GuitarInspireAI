//! Project ids in UUIDv7 layout (project-format section 3). This crate does not read
//! clocks or random sources (FF-03); the caller supplies both.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    /// UUIDv7: 48-bit Unix milliseconds, version 7, variant 10, 74 random bits.
    pub fn from_parts(unix_millis: u64, random: u128) -> Self {
        let ms = (unix_millis & 0xFFFF_FFFF_FFFF) as u128;
        let rand_a = (random >> 64) & 0x0FFF;
        let rand_b = random & 0x3FFF_FFFF_FFFF_FFFF;
        let v = (ms << 80) | (0x7 << 76) | (rand_a << 64) | (0b10 << 62) | rand_b;
        let h = format!("{v:032x}");
        ProjectId(format!(
            "{}-{}-{}-{}-{}",
            &h[0..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..32]
        ))
    }

    /// Keeps the id of a loaded project as written, so saving does not change it.
    pub fn from_loaded(id: String) -> Self {
        ProjectId(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_v7_layout() {
        let id = ProjectId::from_parts(0x0190_0000_0000, u128::MAX);
        let s = id.as_str();
        assert_eq!(s.len(), 36);
        assert_eq!(&s[14..15], "7", "version nibble");
        assert!(matches!(&s[19..20], "8" | "9" | "a" | "b"), "variant bits");
        assert!(s.starts_with("01900000-0000"));
    }
}
