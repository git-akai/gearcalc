//! An exact rational, for the answers that are quotients of integers.
//!
//! **Nothing about gears.** Like [`crate::solve`] and [`crate::hertz`], this
//! knows only its own arithmetic.
//!
//! # Why the kinematics is not floating point
//!
//! A geartrain's ratio *is* a quotient of tooth counts, and this crate already
//! took that decision once, for the hula stage, and wrote down why: an
//! arrangement whose denominator is 1 reduces by exactly `z₂z₄`, and rounding
//! that to a float and back is how a ratio starts disagreeing with the teeth it
//! was counted from. What was true of one arrangement is true of every one, and
//! three more things follow that no tolerance can decide honestly:
//!
//! - **`D = 0` is a refusal, not a large number.** Four of the sixteen hula
//!   arrangements of `z, z ± 1` have their two meshes cancelling exactly, and
//!   the output cannot turn. Against an epsilon that is a threshold somebody
//!   chose; against exact arithmetic it is a zero.
//! - **Mobility is a rank**, and a rank taken with a pivot tolerance is a rank
//!   somebody chose. `docs/corrections.md` — *a bound records where the sweep
//!   stopped* — is the standing warning.
//! - **Lock-up is an identity.** The all-ones vector satisfies every mesh row
//!   exactly or the assembler has a sign wrong; "to within 1e-12" would hide
//!   the very fault the check exists for.
//!
//! # `i128`, and what happens when it is not enough
//!
//! Tooth counts are `u32` and the products a ratio needs are a handful of them,
//! so `i128` holds any train anyone will build. Where it does not, every
//! operation here returns `None` and the caller refuses — an arithmetic that
//! cannot represent the answer says so, rather than wrapping into one that
//! looks like an answer. That is the same treatment
//! [`crate::mesh`] gives a pair that cannot exist.

use std::fmt;

/// A rational number, always in lowest terms with a positive denominator.
///
/// Normalised on construction rather than on comparison, so `==` is structural
/// and two equal numbers cannot be two different values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ratio {
    num: i128,
    den: i128,
}

/// The greatest common divisor, non-negative.
///
/// `i128::MIN` has no positive magnitude, so it is the one input this cannot
/// take; every constructor here rejects it before reaching this.
const fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

impl Ratio {
    /// Zero.
    pub const ZERO: Self = Self { num: 0, den: 1 };
    /// One — and the vector of these is what lock-up is
    /// ([`crate::kinematics`]).
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// `n / d`, in lowest terms. `None` for a zero denominator, or for an
    /// `i128::MIN` that has no positive magnitude to normalise through.
    #[must_use]
    pub fn new(n: i128, d: i128) -> Option<Self> {
        if d == 0 || n == i128::MIN || d == i128::MIN {
            return None;
        }
        let g = gcd(n, d);
        // `g` is zero only when both are, which `d == 0` already refused.
        let (mut num, mut den) = (n / g, d / g);
        if den < 0 {
            num = -num;
            den = -den;
        }
        Some(Self { num, den })
    }

    /// A whole number.
    #[must_use]
    pub const fn whole(n: i64) -> Self {
        Self {
            num: n as i128,
            den: 1,
        }
    }

    /// The numerator, in lowest terms.
    #[must_use]
    pub const fn numerator(self) -> i128 {
        self.num
    }

    /// The denominator, in lowest terms and positive.
    #[must_use]
    pub const fn denominator(self) -> i128 {
        self.den
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    /// `−1`, `0` or `+1`.
    #[must_use]
    pub const fn signum(self) -> i32 {
        if self.num == 0 {
            0
        } else if self.num < 0 {
            -1
        } else {
            1
        }
    }

    #[must_use]
    pub fn checked_neg(self) -> Option<Self> {
        Self::new(self.num.checked_neg()?, self.den)
    }

    /// `1 / self`. `None` at zero, which has no reciprocal.
    #[must_use]
    pub fn recip(self) -> Option<Self> {
        Self::new(self.den, self.num)
    }

    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        let a = self.num.checked_mul(other.den)?;
        let b = other.num.checked_mul(self.den)?;
        Self::new(a.checked_add(b)?, self.den.checked_mul(other.den)?)
    }

    #[must_use]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(other.checked_neg()?)
    }

    #[must_use]
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        // Cancel across the two fractions first, which is what keeps a long
        // elimination inside `i128`: `(a/b)(c/d)` overflows far sooner as
        // `ac/bd` than as the pair of reduced products.
        let g1 = gcd(self.num, other.den);
        let g2 = gcd(other.num, self.den);
        Self::new(
            (self.num / g1).checked_mul(other.num / g2)?,
            (self.den / g2).checked_mul(other.den / g1)?,
        )
    }

    /// `self / other`. `None` at a zero divisor.
    #[must_use]
    pub fn checked_div(self, other: Self) -> Option<Self> {
        self.checked_mul(other.recip()?)
    }

    /// What a reader sees. **Lossy on purpose and one-way**: nothing here reads
    /// a float back.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Ordering, or `None` where the comparison itself would overflow.
    ///
    /// Not `Ord`, because a total order that can fail is not one. Everything
    /// in the solver needs [`Self::is_zero`] and equality, which are exact and
    /// cannot fail; this is for a caller that wants to sort readable output.
    #[must_use]
    pub fn cmp_checked(self, other: Self) -> Option<std::cmp::Ordering> {
        Some(
            self.num
                .checked_mul(other.den)?
                .cmp(&other.num.checked_mul(self.den)?),
        )
    }
}

impl fmt::Display for Ratio {
    /// `3` or `−7/16`. A whole number prints as one, because that is what it
    /// is: the point of the type is that a reduction of exactly 49 says 49.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d).unwrap()
    }

    /// **Equal numbers are the same value.** Normalising on construction is
    /// what lets `==` be structural, which is what lets a nullspace basis be
    /// compared and a zero pivot be recognised without a tolerance.
    #[test]
    fn a_fraction_is_stored_in_lowest_terms_with_a_positive_denominator() {
        assert_eq!(r(6, 8), r(3, 4));
        assert_eq!(r(-6, 8), r(6, -8));
        assert_eq!(r(3, -4).denominator(), 4);
        assert_eq!(r(3, -4).numerator(), -3);
        assert_eq!(r(0, -5), Ratio::ZERO);
        assert_eq!(Ratio::whole(49).to_string(), "49");
        assert_eq!(r(-7, 16).to_string(), "-7/16");
    }

    /// A zero denominator is not a number, and neither is the reciprocal of
    /// zero. Both are refusals rather than infinities.
    #[test]
    fn what_is_not_a_number_is_refused_rather_than_approximated() {
        assert!(Ratio::new(1, 0).is_none());
        assert!(Ratio::ZERO.recip().is_none());
        assert!(Ratio::ONE.checked_div(Ratio::ZERO).is_none());
    }

    /// The field laws, on awkward values rather than tidy ones.
    #[test]
    fn the_arithmetic_is_the_arithmetic_of_fractions() {
        let (a, b, c) = (r(3, 4), r(-5, 6), r(7, 9));
        assert_eq!(a.checked_add(b).unwrap(), r(-1, 12));
        assert_eq!(a.checked_sub(b).unwrap(), r(19, 12));
        assert_eq!(a.checked_mul(b).unwrap(), r(-5, 8));
        assert_eq!(a.checked_div(b).unwrap(), r(-9, 10));
        // Associativity and distributivity, which a normalisation bug breaks.
        assert_eq!(
            a.checked_add(b).unwrap().checked_add(c).unwrap(),
            a.checked_add(b.checked_add(c).unwrap()).unwrap()
        );
        assert_eq!(
            a.checked_mul(b.checked_add(c).unwrap()).unwrap(),
            a.checked_mul(b)
                .unwrap()
                .checked_add(a.checked_mul(c).unwrap())
                .unwrap()
        );
        assert_eq!(a.checked_mul(a.recip().unwrap()).unwrap(), Ratio::ONE);
        assert_eq!(a.checked_sub(a).unwrap(), Ratio::ZERO);
    }

    /// **The hula stage's own numbers, exactly.** `z₂z₄ = 61·61 = 3721` and
    /// `D = 3721 − 65·57 = 16`, so the shipped arrangement reduces by
    /// `3721/16` and not by 232.5625 — and an arrangement whose `D` is zero has
    /// no ratio at all rather than a very large one.
    #[test]
    fn a_reduction_is_the_two_products_rather_than_their_quotient() {
        let reduction = |z: [i128; 4]| {
            let (p, q) = (z[1] * z[3], z[0] * z[2]);
            Ratio::new(p, p - q)
        };
        let shipped = reduction([65, 61, 57, 61]).unwrap();
        assert_eq!(shipped.numerator(), 3721);
        assert_eq!(shipped.denominator(), 16);
        assert!((shipped.to_f64() - 232.5625).abs() < 1e-12);
        // `z₂z₄ = z₁z₃` — the two meshes step by the same amount and cancel.
        assert!(reduction([60, 60, 60, 60]).is_none());
    }

    /// **Overflow is a refusal, not a wrap.** The one way exact arithmetic can
    /// go wrong silently, and the reason every operation returns an `Option`.
    #[test]
    fn an_answer_too_big_to_represent_is_refused() {
        let huge = Ratio::whole(i64::MAX);
        assert!(
            huge.checked_mul(huge).is_some(),
            "two i64s still fit in an i128"
        );
        // ...and four do not.
        let big = huge.checked_mul(huge).unwrap();
        assert!(big.checked_mul(big).is_none());
        assert!(Ratio::new(i128::MAX, 1)
            .unwrap()
            .checked_add(Ratio::ONE)
            .is_none());
    }

    /// Cancelling across the two fractions before multiplying is what keeps a
    /// long elimination inside `i128`; the same product written out whole
    /// overflows.
    #[test]
    fn multiplication_cancels_before_it_multiplies() {
        let a = Ratio::new(i128::MAX / 3, 7).unwrap();
        let b = Ratio::new(7, i128::MAX / 3).unwrap();
        assert_eq!(a.checked_mul(b).unwrap(), Ratio::ONE);
    }

    /// Ordering is offered where it is exact and withheld where it is not,
    /// rather than falling back on a float.
    #[test]
    fn comparison_is_exact_or_absent() {
        use std::cmp::Ordering;
        assert_eq!(r(1, 3).cmp_checked(r(1, 2)), Some(Ordering::Less));
        assert_eq!(r(-1, 3).cmp_checked(r(-1, 2)), Some(Ordering::Greater));
        assert_eq!(r(2, 4).cmp_checked(r(1, 2)), Some(Ordering::Equal));
        // Two numbers a hair apart, each too large for the cross-multiplication
        // that would order them. Withheld rather than answered from a float,
        // which is the one place a tolerance could have crept into the module.
        let edge = Ratio::new(i128::MAX, 2).unwrap();
        assert_eq!(edge.cmp_checked(Ratio::new(i128::MAX, 3).unwrap()), None);
    }
}
