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

    /// `eˣ`: `x = k ln 2 + r`, `e^r` by its Taylor series at `r/2¹⁰`, squared ten times.
    #[expect(clippy::cast_possible_truncation, reason = "k is a small exponent")]
    pub fn exp(self) -> Self {
        let k = (self.hi / LN_2.hi).round();
        let r = (self - LN_2 * Self::of(k)).scale(-10);
        // e^r − 1 by its series, |r| ≤ ln 2 / 2¹¹: 12 terms put the next below 1e-33.
        let mut term = r;
        let mut sum = r;
        for n in 2..=12 {
            term = term * r / Self::of(f64::from(n));
            sum = sum + term;
        }
        // (1 + s)² − 1 = s (2 + s), ten times.
        for _ in 0..10 {
            sum = sum * (sum + Self::of(2.0));
        }
        (sum + Self::of(1.0)).scale(k as i32)
    }

    /// `ln x` by Newton on `eʸ = x` from the `f64` logarithm: two steps double 16 digits twice.
    pub fn ln(self) -> Self {
        let mut y = Self::of(self.hi.ln());
        for _ in 0..2 {
            y = y + self * (-y).exp() - Self::of(1.0);
        }
        y
    }

    /// `asinh x = ln(|x| + √(x² + 1))`, odd.
    pub fn asinh(self) -> Self {
        let a = self.abs();
        let v = (a + (a * a + Self::of(1.0)).sqrt()).ln();
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

    pub fn sinh(self) -> Self {
        let e = self.exp();
        (e - Self::of(1.0) / e).scale(-1)
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
    /// and `sin π = 0` by its series (the constant π). A plant: π's low word dropped fails the
    /// last, and ln 2's the one before.
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
        let crude = Wide::of(std::f64::consts::PI);
        assert!(sin(crude).abs().to_f64() > 1e-17);
        let crude_ln2 = Wide::of(std::f64::consts::LN_2);
        assert!(!close(crude_ln2.exp(), two));
    }
}
