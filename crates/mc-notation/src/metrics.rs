//! Layout metrics supplied by the UI (ADR-018). Dart measures the bundled TAB font
//! (ADR-021) once per zoom level and passes the numbers; Rust never renders text.
//! All values are abstract units (logical pixels at the current zoom).

use crate::NotationError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutMetrics {
    /// Width of one system line, including the margins.
    pub width: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    /// Vertical distance between two string lines.
    pub string_spacing: f32,
    /// Advance width of one digit in the TAB font.
    pub digit_width: f32,
    /// Free space on each side of a fret number.
    pub glyph_padding: f32,
    /// Width of a column whose duration is a sixteenth note.
    pub min_column_width: f32,
    /// Extra column width per doubling of the duration, as a fraction of
    /// `min_column_width` (0 gives equal spacing).
    pub duration_spacing: f32,
    /// Space inside a bar before the first and after the last column.
    pub bar_padding: f32,
    /// Width of a bar without beats.
    pub empty_bar_width: f32,
    /// TAB clef and string names at the start of every system.
    pub header_width: f32,
    pub time_signature_width: f32,
    /// Extra width taken by a repeat start or repeat end sign.
    pub repeat_width: f32,
    /// Space above the top string (voltas, markers, upper-voice rhythm).
    pub above_staff: f32,
    /// Space below the bottom string (rhythm stems and beams).
    pub below_staff: f32,
    /// Vertical gap between systems.
    pub system_gap: f32,
    pub stem_length: f32,
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        LayoutMetrics {
            width: 1000.0,
            margin_left: 24.0,
            margin_right: 24.0,
            string_spacing: 12.0,
            digit_width: 7.0,
            glyph_padding: 2.0,
            min_column_width: 18.0,
            duration_spacing: 0.4,
            bar_padding: 8.0,
            empty_bar_width: 80.0,
            header_width: 28.0,
            time_signature_width: 16.0,
            repeat_width: 10.0,
            above_staff: 28.0,
            below_staff: 36.0,
            system_gap: 16.0,
            stem_length: 20.0,
        }
    }
}

impl LayoutMetrics {
    /// Width available for bars in one system.
    pub fn content_width(&self) -> f32 {
        self.width - self.margin_left - self.margin_right - self.header_width
    }

    pub fn validate(&self) -> Result<(), NotationError> {
        let positive = [
            ("width", self.width),
            ("string_spacing", self.string_spacing),
            ("digit_width", self.digit_width),
            ("min_column_width", self.min_column_width),
            ("empty_bar_width", self.empty_bar_width),
            ("stem_length", self.stem_length),
        ];
        let non_negative = [
            ("margin_left", self.margin_left),
            ("margin_right", self.margin_right),
            ("glyph_padding", self.glyph_padding),
            ("duration_spacing", self.duration_spacing),
            ("bar_padding", self.bar_padding),
            ("header_width", self.header_width),
            ("time_signature_width", self.time_signature_width),
            ("repeat_width", self.repeat_width),
            ("above_staff", self.above_staff),
            ("below_staff", self.below_staff),
            ("system_gap", self.system_gap),
        ];
        for (name, v) in positive {
            if !v.is_finite() || v <= 0.0 {
                return Err(NotationError::InvalidMetrics(format!("{name} must be > 0")));
            }
        }
        for (name, v) in non_negative {
            if !v.is_finite() || v < 0.0 {
                return Err(NotationError::InvalidMetrics(format!(
                    "{name} must be >= 0"
                )));
            }
        }
        if self.content_width() < self.empty_bar_width {
            return Err(NotationError::InvalidMetrics(
                "width minus margins and header is narrower than an empty bar".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_bad_values_are_rejected() {
        assert_eq!(LayoutMetrics::default().validate(), Ok(()));
        for m in [
            LayoutMetrics {
                width: 100.0,
                ..LayoutMetrics::default()
            },
            LayoutMetrics {
                digit_width: 0.0,
                ..LayoutMetrics::default()
            },
            LayoutMetrics {
                bar_padding: f32::NAN,
                ..LayoutMetrics::default()
            },
            LayoutMetrics {
                margin_left: -1.0,
                ..LayoutMetrics::default()
            },
        ] {
            assert!(m.validate().is_err(), "{m:?}");
        }
    }
}
