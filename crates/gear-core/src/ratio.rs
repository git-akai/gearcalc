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

    /// **A double's exact value**: every finite double is `m·2^e` with `m` a
    /// 53-bit integer, so it is a rational, and this is that rational with no
    /// rounding. `None` where it is not finite, or where its numerator or
    /// denominator needs more than an `i128` holds: any number from `2¹²⁷`,
    /// and one below about `2⁻⁷⁴` whose mantissa uses all its bits.
    ///
    /// For a figure a designer gave, read where the question is exact — two
    /// speeds given on one rigid chain agree or they do not.
    #[must_use]
    pub fn of_double(x: f64) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        if x == 0.0 {
            return Some(Self::ZERO);
        }
        let (magnitude, exponent) = dyadic(x)?;
        let mut mantissa = i128::from(magnitude);
        if x < 0.0 {
            mantissa = -mantissa;
        }
        let power = |e: i32| -> Option<i128> {
            1_i128
                .checked_shl(u32::try_from(e).ok()?)
                .filter(|p| *p > 0)
        };
        if exponent >= 0 {
            Self::new(mantissa.checked_mul(power(exponent)?)?, 1)
        } else {
            Self::new(mantissa, power(-exponent)?)
        }
    }

    /// **The least whole number not below this**, exactly: a count of
    /// whole things that this many, or a part more, takes.
    #[must_use]
    pub fn ceil(self) -> i128 {
        // The denominator is positive in lowest terms.
        -((-self.num).div_euclid(self.den))
    }

    /// The magnitude. `None` for the one value with none in `i128`, which
    /// [`Self::new`] never makes.
    #[must_use]
    pub fn checked_abs(self) -> Option<Self> {
        if self.num < 0 {
            self.checked_neg()
        } else {
            Some(self)
        }
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

    /// What a reader sees. **Lossy on purpose**: the one way a float comes
    /// back is [`Self::of_double`], which reads a designer's figure exactly.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// **`x` times this ratio**, rounded as few times as the arithmetic allows.
    ///
    /// `x * self.to_f64()` rounds twice — once for the quotient and once for
    /// the product — and `(x * num) / den` rounds **once** whenever `x · num`
    /// is exactly representable, which it is for the small integer numerators a
    /// tooth count makes. Exact arithmetic is worth having only if it is spent
    /// last, and this is where a speed stops being exact.
    ///
    /// Measured: a member's speed came back one ULP from the correctly rounded
    /// value when the ratio was converted first, and on the correctly rounded
    /// value when the multiplication came first.
    #[must_use]
    pub fn scale(self, x: f64) -> f64 {
        x * (self.num as f64) / (self.den as f64)
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

/// **A double's magnitude as `m · 2^e`**, exactly: every finite double is
/// a dyadic rational, the mantissa an odd integer once its trailing zeros
/// are taken into the exponent. `None` for one that is not finite; `(0, 0)`
/// for nought.
#[must_use]
pub fn dyadic(x: f64) -> Option<(u64, i32)> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some((0, 0));
    }
    let bits = x.abs().to_bits();
    let biased = i32::try_from(bits >> 52).ok()?;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (mantissa, exponent) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1_u64 << 52), biased - 1075)
    };
    let zeros = mantissa.trailing_zeros();
    Some((mantissa >> zeros, exponent + i32::try_from(zeros).ok()?))
}

/// **A natural number of any size**, as little-endian 64-bit limbs — what an
/// exact count needs where its numerator or denominator is past an `i128`:
/// a duration's `2^e` on a speed's quotient. Products, sums, differences,
/// shifts and one quotient; nothing else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Natural(Vec<u64>);

// A limb is the low 64 bits of a wider product, taken on purpose.
#[allow(clippy::cast_possible_truncation)]
impl Natural {
    /// `n`.
    #[must_use]
    pub fn of(n: u128) -> Self {
        Self(vec![n as u64, (n >> 64) as u64]).trimmed()
    }

    fn trimmed(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    /// Whether this is nought.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// `self · n`.
    #[must_use]
    pub fn times(&self, n: u128) -> Self {
        let parts = [n as u64, (n >> 64) as u64];
        let mut out = vec![0_u64; self.0.len() + 2];
        for (j, &p) in parts.iter().enumerate() {
            let mut carry = 0_u128;
            for (i, &a) in self.0.iter().enumerate() {
                let t = u128::from(a) * u128::from(p) + u128::from(out[i + j]) + carry;
                out[i + j] = t as u64;
                carry = t >> 64;
            }
            let mut k = self.0.len() + j;
            while carry > 0 {
                let t = u128::from(out[k]) + carry;
                out[k] = t as u64;
                carry = t >> 64;
                k += 1;
            }
        }
        Self(out).trimmed()
    }

    /// `self + other`.
    #[must_use]
    pub fn plus(&self, other: &Self) -> Self {
        let n = self.0.len().max(other.0.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0_u128;
        for i in 0..n {
            let t = u128::from(self.0.get(i).copied().unwrap_or_default())
                + u128::from(other.0.get(i).copied().unwrap_or_default())
                + carry;
            out.push(t as u64);
            carry = t >> 64;
        }
        out.push(carry as u64);
        Self(out).trimmed()
    }

    /// `|self − other|`.
    #[must_use]
    pub fn distance(&self, other: &Self) -> Self {
        let (big, small) = if self >= other {
            (self, other)
        } else {
            (other, self)
        };
        let mut out = Vec::with_capacity(big.0.len());
        let mut borrow = false;
        for i in 0..big.0.len() {
            let b = small.0.get(i).copied().unwrap_or_default();
            let (d, o1) = big.0[i].overflowing_sub(b);
            let (d, o2) = d.overflowing_sub(u64::from(borrow));
            out.push(d);
            borrow = o1 || o2;
        }
        Self(out).trimmed()
    }

    /// `self · 2^bits`.
    #[must_use]
    pub fn shifted(&self, bits: u32) -> Self {
        let (limbs, rest) = ((bits / 64) as usize, bits % 64);
        let mut out = vec![0_u64; limbs];
        let mut carry = 0_u64;
        for &a in &self.0 {
            out.push((a << rest) | carry);
            carry = if rest == 0 { 0 } else { a >> (64 - rest) };
        }
        out.push(carry);
        Self(out).trimmed()
    }

    /// **`⌈self / by⌉`, where it is below `2^53`** — every whole number a
    /// double holds exactly, which is the most a report printed as one can
    /// say. `None` where the quotient is that large or more, or `by` is
    /// nought.
    #[must_use]
    pub fn ceil_over(&self, by: &Self) -> Option<u64> {
        const EXACT_BITS: u32 = f64::MANTISSA_DIGITS;
        if by.is_zero() || *self >= by.shifted(EXACT_BITS) {
            return None;
        }
        // Long division, one bit of the quotient at a time from the top.
        let (mut rest, mut q) = (self.clone(), 0_u64);
        for bit in (0..EXACT_BITS).rev() {
            let part = by.shifted(bit);
            if rest >= part {
                rest = rest.distance(&part);
                q |= 1 << bit;
            }
        }
        Some(q + u64::from(!rest.is_zero()))
    }
}

impl PartialOrd for Natural {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Natural {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// **A natural's quotient is the integers' own**: over a grid of
    /// numerators, divisors and shifts that `u128` still holds, `ceil_over`
    /// is `u128`'s ceiling exactly; a quotient of `2^53` is past it and none;
    /// and a dyadic reads back as the double it came from.
    #[test]
    fn a_naturals_ceiling_is_the_integers() {
        let mut read = 0;
        for n in [0_u128, 1, 2, 7, 48_000_000, (1 << 60) + 3, (1 << 100) - 1] {
            for d in [1_u128, 3, 7, 1 << 40, (1 << 47) + 1] {
                for shift in [0_u32, 5, 13] {
                    let (num, den) = (Natural::of(n).shifted(shift), Natural::of(d));
                    let exact = (n << shift).div_ceil(d);
                    let got = num.ceil_over(&den);
                    if exact < 1 << 53 {
                        assert_eq!(got, u64::try_from(exact).ok(), "{n}·2^{shift} / {d}");
                    } else {
                        assert_eq!(got, None, "{n}·2^{shift} / {d}");
                    }
                    read += 1;
                }
            }
        }
        assert_eq!(read, 105);
        // The edge: a quotient one under 2^53 reads, 2^53 does not.
        let edge = Natural::of(1 << 53);
        assert_eq!(edge.ceil_over(&Natural::of(1)), None);
        assert_eq!(
            Natural::of((1 << 53) - 1).ceil_over(&Natural::of(1)),
            Some((1 << 53) - 1)
        );
        // Sums, products and differences past 128 bits come back.
        let big = Natural::of(u128::MAX).times(u128::MAX);
        assert_eq!(big.distance(&big), Natural::of(0));
        assert_eq!(big.plus(&Natural::of(1)).distance(&big), Natural::of(1));
        for x in [
            1.0,
            0.1,
            1000.0_f64.next_up(),
            2f64.powi(126),
            1e-30,
            5e-324,
        ] {
            let (m, e) = dyadic(x).unwrap();
            assert_eq!(m as f64 * 2f64.powi(e), x, "{x}");
        }
    }

    fn r(n: i128, d: i128) -> Ratio {
        Ratio::new(n, d).unwrap()
    }

    /// **A double read exactly**: its value as a rational, with no rounding —
    /// so it reads back to the same double — and `None` where the rational
    /// does not fit or the double is not a number.
    #[test]
    fn a_double_is_read_as_the_rational_it_is() {
        assert_eq!(Ratio::of_double(4300.0), Some(Ratio::whole(4300)));
        assert_eq!(Ratio::of_double(-1700.0), Some(Ratio::whole(-1700)));
        assert_eq!(Ratio::of_double(0.0), Some(Ratio::ZERO));
        assert_eq!(Ratio::of_double(-0.375), Some(r(-3, 8)));
        // 0.1 is not a tenth: it is the nearest double, exactly.
        assert_eq!(
            Ratio::of_double(0.1),
            Some(r(3_602_879_701_896_397, 36_028_797_018_963_968))
        );
        let mut read = 0;
        for x in [1.0, 1e-20, 123.456, 2e30, -7.0 / 3.0, f64::EPSILON, 1e37] {
            let q = Ratio::of_double(x).unwrap();
            assert_eq!(q.to_f64().to_bits(), x.to_bits(), "{x}");
            read += 1;
        }
        assert_eq!(read, 7);
        // Past what an i128 holds either way, and not numbers at all.
        for x in [1e39, 1e-30, 5e-324, f64::MAX, f64::NAN, f64::INFINITY] {
            assert_eq!(Ratio::of_double(x), None, "{x}");
        }
        // The edges: 2¹²⁶ fits, 2¹²⁷ does not; 2⁻¹²⁶ fits, 2⁻¹²⁷ does not.
        assert!(Ratio::of_double(2f64.powi(126)).is_some());
        assert_eq!(Ratio::of_double(2f64.powi(127)), None);
        assert!(Ratio::of_double(2f64.powi(-126)).is_some());
        assert_eq!(Ratio::of_double(2f64.powi(-127)), None);
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
