use crate::TheoryError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Major,
    Minor,
}

/// Key signature as a position on the circle of fifths: -7 (seven flats) to +7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeySignature {
    fifths: i8,
    mode: Mode,
}

impl KeySignature {
    pub const C_MAJOR: KeySignature = KeySignature {
        fifths: 0,
        mode: Mode::Major,
    };

    pub fn new(fifths: i8, mode: Mode) -> Result<Self, TheoryError> {
        if !(-7..=7).contains(&fifths) {
            return Err(TheoryError::InvalidKey(fifths));
        }
        Ok(KeySignature { fifths, mode })
    }

    pub const fn fifths(self) -> i8 {
        self.fifths
    }

    pub const fn mode(self) -> Mode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_range() {
        assert!(KeySignature::new(7, Mode::Major).is_ok());
        assert!(KeySignature::new(-7, Mode::Minor).is_ok());
        assert_eq!(
            KeySignature::new(8, Mode::Major),
            Err(TheoryError::InvalidKey(8))
        );
    }
}
