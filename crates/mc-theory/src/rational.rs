//! Exact rational numbers for musical time (ADR-011). No floats in the music model (T-01).

use crate::TheoryError;
use std::cmp::Ordering;
use std::fmt;

/// Normalized fraction: denominator > 0 and gcd(numerator, denominator) == 1.
/// Arithmetic is checked; overflow is reported instead of wrapping.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i64,
    den: i64,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Rational {
    pub const ZERO: Rational = Rational { num: 0, den: 1 };
    pub const ONE: Rational = Rational { num: 1, den: 1 };

    pub fn new(num: i64, den: i64) -> Result<Self, TheoryError> {
        if den == 0 {
            return Err(TheoryError::ZeroDenominator);
        }
        Self::from_i128(num as i128, den as i128).ok_or(TheoryError::Overflow)
    }

    pub const fn from_int(n: i64) -> Self {
        Rational { num: n, den: 1 }
    }

    fn from_i128(num: i128, den: i128) -> Option<Self> {
        let (mut n, mut d) = (num, den);
        if d < 0 {
            n = -n;
            d = -d;
        }
        let g = gcd(n, d).max(1);
        n /= g;
        d /= g;
        Some(Rational {
            num: i64::try_from(n).ok()?,
            den: i64::try_from(d).ok()?,
        })
    }

    pub const fn numerator(self) -> i64 {
        self.num
    }

    pub const fn denominator(self) -> i64 {
        self.den
    }

    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    pub const fn is_positive(self) -> bool {
        self.num > 0
    }

    pub const fn is_negative(self) -> bool {
        self.num < 0
    }

    pub fn checked_add(self, o: Self) -> Option<Self> {
        let n = self.num as i128 * o.den as i128 + o.num as i128 * self.den as i128;
        Self::from_i128(n, self.den as i128 * o.den as i128)
    }

    pub fn checked_sub(self, o: Self) -> Option<Self> {
        let n = self.num as i128 * o.den as i128 - o.num as i128 * self.den as i128;
        Self::from_i128(n, self.den as i128 * o.den as i128)
    }

    pub fn checked_mul(self, o: Self) -> Option<Self> {
        Self::from_i128(
            self.num as i128 * o.num as i128,
            self.den as i128 * o.den as i128,
        )
    }

    pub fn checked_div(self, o: Self) -> Option<Self> {
        if o.num == 0 {
            return None;
        }
        Self::from_i128(
            self.num as i128 * o.den as i128,
            self.den as i128 * o.num as i128,
        )
    }

    /// Sum of an iterator of rationals; `None` on overflow.
    pub fn checked_sum<I: IntoIterator<Item = Rational>>(items: I) -> Option<Self> {
        items
            .into_iter()
            .try_fold(Rational::ZERO, Rational::checked_add)
    }
}

impl Ord for Rational {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.num as i128 * o.den as i128).cmp(&(o.num as i128 * self.den as i128))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn r(n: i64, d: i64) -> Rational {
        Rational::new(n, d).unwrap()
    }

    #[test]
    fn normalizes_sign_and_gcd() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(1, -2), r(-1, 2));
        assert_eq!(r(-3, -9), r(1, 3));
        assert_eq!(r(0, 5), Rational::ZERO);
        assert_eq!(r(0, 5).denominator(), 1);
    }

    #[test]
    fn zero_denominator_is_rejected() {
        assert_eq!(Rational::new(1, 0), Err(TheoryError::ZeroDenominator));
    }

    #[test]
    fn arithmetic_and_ordering() {
        assert_eq!(r(1, 3).checked_add(r(1, 6)), Some(r(1, 2)));
        assert_eq!(r(1, 2).checked_sub(r(3, 4)), Some(r(-1, 4)));
        assert_eq!(r(2, 3).checked_mul(r(3, 4)), Some(r(1, 2)));
        assert_eq!(r(1, 2).checked_div(r(1, 4)), Some(r(2, 1)));
        assert_eq!(r(1, 2).checked_div(Rational::ZERO), None);
        assert!(r(1, 3) < r(1, 2));
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        let big = Rational::from_int(i64::MAX);
        assert_eq!(big.checked_add(Rational::ONE), None);
        assert_eq!(big.checked_mul(Rational::from_int(2)), None);
    }

    proptest! {
        // T-01: exact arithmetic properties used by timing code.
        #[test]
        fn t01_add_sub_roundtrip(a in -10_000i64..10_000, b in 1i64..10_000, c in -10_000i64..10_000, d in 1i64..10_000) {
            let x = r(a, b);
            let y = r(c, d);
            prop_assert_eq!(x.checked_add(y).unwrap().checked_sub(y).unwrap(), x);
            prop_assert_eq!(x.checked_add(y), y.checked_add(x));
        }

        #[test]
        fn t01_always_normalized(a in any::<i32>(), b in 1i32..i32::MAX) {
            let x = r(a as i64, b as i64);
            prop_assert!(x.denominator() > 0);
            prop_assert_eq!(gcd(x.numerator() as i128, x.denominator() as i128).max(1), 1);
        }
    }
}
