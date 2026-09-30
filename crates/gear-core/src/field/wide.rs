//! Double-double numbers for the field's references: a value is `hi + lo`, `|lo| ≤ ulp(hi)/2`,
//! about 32 significant digits. Each operation is the standard error-free construction (Dekker;
//! Knuth's two-sum; the product's error by a fused multiply-add), and each function a Newton
//! step or a series on it, so a reference built here shares no code with the `f64` it checks.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// `hi + lo`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Wide {
    pub hi: f64,
    pub lo: f64,
}

/// `a + b` exactly, as `(s, e)` with `s = fl(a + b)` (Knuth).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let v = s - a;
    (s, (a - (s - v)) + (b - v))
}

/// `a + b` exactly where `|a| ≥ |b|` (Dekker).
fn quick_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}

/// `a b` exactly: the product's rounding error is `fma(a, b, −p)`.
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    (p, a.mul_add(b, -p))
}

/// π to 32 digits.
const PI: Wide = Wide {
    hi: std::f64::consts::PI,
    lo: 1.224_646_799_147_353_2e-16,
};

/// ln 2 to 32 digits.
const LN_2: Wide = Wide {
    hi: std::f64::consts::LN_2,
    lo: 2.319_046_813_846_299_6e-17,
};

impl Wide {
    pub const fn of(x: f64) -> Self {
        Self { hi: x, lo: 0.0 }
    }

    pub fn pi() -> Self {
        PI
    }

    fn norm(hi: f64, lo: f64) -> Self {
        let (hi, lo) = quick_two_sum(hi, lo);
        Self { hi, lo }
    }

    /// The nearest `f64`.
    pub fn to_f64(self) -> f64 {
        self.hi + self.lo
    }

    pub fn abs(self) -> Self {
        if self.hi < 0.0 {
            -self
        } else {
            self
        }
    }

    /// `x · 2ᵏ`, exact.
    fn scale(self, k: i32) -> Self {
        let f = 2.0_f64.powi(k);
        Self {
            hi: self.hi * f,
            lo: self.lo * f,
        }
    }

    /// `√x` by one Newton step on the `f64` root (Karp): `q + (x − q²)/(2q)`.
    pub fn sqrt(self) -> Self {
        if self.hi <= 0.0 {
            return Self::of(0.0);
        }
        let q = self.hi.sqrt();
        let y = Self::of(q);
        y + (self - y * y) * Self::of(0.5 / q)
    }

    /// `eˣ`: `x = k ln 2 + r`, `e^r` from [`Wide::exp_m1_reduced`].
    #[expect(clippy::cast_possible_truncation, reason = "k is a small exponent")]
    pub fn exp(self) -> Self {
        let k = (self.hi / LN_2.hi).round();
        let r = self - LN_2 * Self::of(k);
        (r.exp_m1_reduced() + Self::of(1.0)).scale(k as i32)
    }

    /// `eˣ − 1` for `|x| ≤ ln 2 / 2`, without cancellation: its series at `x/2¹⁰`, then
    /// `(1 + s)² − 1 = s (2 + s)` ten times.
    fn exp_m1_reduced(self) -> Self {
        let r = self.scale(-10);
        // |r| ≤ ln 2 / 2¹¹: 12 terms put the next below 1e-33 of the sum.
        let mut term = r;
        let mut sum = r;
        for n in 2..=12 {
            term = term * r / Self::of(f64::from(n));
            sum = sum + term;
        }
        for _ in 0..10 {
            sum = sum * (sum + Self::of(2.0));
        }
        sum
    }

    /// `eˣ − 1`, to 32 digits of itself however small `x` is.
    pub fn exp_m1(self) -> Self {
        if self.hi.abs() <= LN_2.hi / 2.0 {
            self.exp_m1_reduced()
        } else {
            self.exp() - Self::of(1.0)
        }
    }

    /// `ln x` by Newton on `eʸ = x` from the `f64` logarithm: two steps double 16 digits twice.
    pub fn ln(self) -> Self {
        let mut y = Self::of(self.hi.ln());
        for _ in 0..2 {
            y = y + self * (-y).exp() - Self::of(1.0);
        }
        y
    }

    /// `ln(1 + x)`, to 32 digits of itself however small `x` is: Newton on `e^y − 1 = x`
    /// ([`Wide::exp_m1`]) from the `f64` value, two steps doubling 16 digits twice.
    pub fn ln_1p(self) -> Self {
        let mut y = Self::of(self.hi.ln_1p());
        for _ in 0..2 {
            let m = y.exp_m1();
            y = y + (self - m) / (m + Self::of(1.0));
        }
        y
    }

    /// `asinh x = ln(1 + |x| + x²/(1 + √(x² + 1)))`, odd: every term of one sign, so to 32
    /// digits of itself at small `x` as at large.
    pub fn asinh(self) -> Self {
        let a = self.abs();
        let one = Self::of(1.0);
        let v = (a + a * a / (one + (a * a + one).sqrt())).ln_1p();
        if self.hi < 0.0 {
            -v
        } else {
            v
        }
    }

    pub fn cosh(self) -> Self {
        let e = self.exp();
        (e + Self::of(1.0) / e).scale(-1)
    }

    /// `sinh x = (m + m/(1 + m))/2`, `m = e^|x| − 1`: a sum of two terms of one sign, so to 32
    /// digits of itself however small `x` is (`(eˣ − e⁻ˣ)/2` differences two values near 1, and
    /// keeps about 16 digits at `x ≈ 1e-16`). Odd.
    pub fn sinh(self) -> Self {
        let m = self.abs().exp_m1();
        let v = (m + m / (m + Self::of(1.0))).scale(-1);
        if self.hi < 0.0 {
            -v
        } else {
            v
        }
    }
}

impl Add for Wide {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        let (s, e) = two_sum(self.hi, o.hi);
        let (t, f) = two_sum(self.lo, o.lo);
        let (s, e) = quick_two_sum(s, e + t);
        Self::norm(s, e + f)
    }
}

impl Neg for Wide {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            hi: -self.hi,
            lo: -self.lo,
        }
    }
}

impl Sub for Wide {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        self + (-o)
    }
}

impl Mul for Wide {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        let (p, e) = two_prod(self.hi, o.hi);
        Self::norm(p, e + (self.hi * o.lo + self.lo * o.hi))
    }
}

impl Div for Wide {
    type Output = Self;
    /// Long division, three `f64` quotient digits.
    fn div(self, o: Self) -> Self {
        let q1 = self.hi / o.hi;
        let r = self - o * Self::of(q1);
        let q2 = r.hi / o.hi;
        let r = r - o * Self::of(q2);
        let q3 = r.hi / o.hi;
        Self::norm(q1, q2) + Self::of(q3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants and the functions against identities they must satisfy to 32 digits:
    /// `(√2)² = 2`, `e^{ln 3} = 3`, `ln(e) = 1` through `exp(1)`, `asinh(sinh 0.7) = 0.7`,
    /// `cosh² − sinh² = 1`, `(1/3)·3 = 1`, `e^{ln 2} = 2` (the constant ln 2 against `exp`),
    /// `sin π = 0` by its series (the constant π), and `sinh`, `eˣ − 1` and `asinh` against their
    /// series at small arguments, `ln(1 + x)` against `eˣ − 1` there, `ln_1p(e − 1) = 1` and
    /// `asinh 10⁴⁰ = ln(2·10⁴⁰)`. Plants: π's low word dropped fails `sin π`, ln 2's
    /// `e^{ln 2}`, and `sinh` as `(eˣ − e⁻ˣ)/2` three of the five small arguments.
    #[test]
    fn the_functions_hold_their_identities_to_32_digits() {
        let tol = 1e-30;
        let one = Wide::of(1.0);
        let close = |a: Wide, b: Wide| (a - b).abs().to_f64() <= tol * b.abs().to_f64().max(1.0);
        let two = Wide::of(2.0);
        let r2 = two.sqrt();
        assert!(close(r2 * r2, two));
        assert!(close(Wide::of(3.0).ln().exp(), Wide::of(3.0)));
        assert!(close(one.exp().ln(), one));
        let x = Wide::of(0.7);
        assert!(close(x.sinh().asinh(), x));
        assert!(close(x.cosh() * x.cosh() - x.sinh() * x.sinh(), one));
        assert!(close(one / Wide::of(3.0) * Wide::of(3.0), one));
        assert!(close(LN_2.exp(), two));
        // sin x = x − x³/3! + …, 40 terms at x = π: the tail is below 1e-40.
        let sin = |x: Wide| {
            let (mut term, mut sum) = (x, x);
            for k in 1..40 {
                let d = f64::from(2 * k) * f64::from(2 * k + 1);
                term = -(term * x * x) / Wide::of(d);
                sum = sum + term;
            }
            sum
        };
        assert!(sin(PI).abs().to_f64() < tol);
        // sinh x = x (1 + x²/6 + …) and eˣ − 1 = x (1 + x/2 + …), to 32 digits of themselves at
        // small x; asinh undoes sinh there. A plant: (eˣ − e⁻ˣ)/2, whose eˣ holds 1 + x to 32
        // digits of 1, not of x: it misses from 1e-8 to 1e-16 (4.6e-17 of itself there), and
        // holds only where x's square falls below x's own low word.
        let of_itself = |a: Wide, b: Wide| (a - b).abs().to_f64() <= tol * b.abs().to_f64();
        let (mut small, mut naive_missed) = (0, 0);
        for x in [1e-40, -3e-25, 1e-16, 1e-12, 1e-8] {
            let w = Wide::of(x);
            let series = w * (one + w * w / Wide::of(6.0));
            assert!(of_itself(w.sinh(), series), "sinh {x}");
            let m1 = w * (one + w / two + w * w / Wide::of(6.0) + w * w * w / Wide::of(24.0));
            assert!(of_itself(w.exp_m1(), m1), "exp_m1 {x}");
            assert!(of_itself(w.sinh().asinh(), w), "asinh sinh {x}");
            assert!(of_itself((-w).exp_m1().ln_1p(), -w), "ln_1p {x}");
            let naive = (w.exp() - one / w.exp()).scale(-1);
            naive_missed += usize::from(!of_itself(naive, series));
            small += 1;
        }
        assert_eq!((small, naive_missed), (5, 3));
        assert!(close(x.exp_m1() + one, x.exp()));
        assert!(close(Wide::of(1e40).asinh(), (Wide::of(2e40)).ln()));
        assert!(close((one.exp() - one).ln_1p(), one));
        let crude = Wide::of(std::f64::consts::PI);
        assert!(sin(crude).abs().to_f64() > 1e-17);
        let crude_ln2 = Wide::of(std::f64::consts::LN_2);
        assert!(!close(crude_ln2.exp(), two));
    }
}
