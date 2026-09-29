//! JGMA 116-02 (1983) double-flank composite tolerances.
//!
//! The standard is a **banded lookup table, not a formula**: find the module
//! band, find the pitch-diameter band, read two numbers in micrometres. Nothing
//! is interpolated, because the standard does not interpolate.
//!
//! # Two scales, not one
//!
//! The document contains two tables, and they are **independent grade scales**.
//! A grade number on one is not the same tolerance as the same number on the
//! other. At module (1, 1.6] and a 12 mm pitch diameter, where both tables apply:
//!
//! | Grade | fine | standard |
//! |---|---|---|
//! | 0 | 6.3 / 18 | — |
//! | 3 | 14 / 53 | — |
//! | 4 | 22 / 71 | **7 / 20** |
//! | 6 | 45 / 140 | 14 / 50 |
//!
//! The standard scale's grade 4 is three times tighter than the fine scale's.
//! Merge the two by taking the smaller value at each grade and the ladder reads
//! 6.3, 8, 10, 14, **7**, 10, 14, … — it *drops* between grade 3 and grade 4.
//! No rule for choosing between overlapping entries avoids this, because the
//! grade numbers simply do not denote the same thing on the two tables.
//!
//! They are therefore kept separate and never compared. The document's own
//! annotation supports this: the fine table's (1, 1.6] column is marked 選用
//! (*optional*), while its finer columns are 適用 (*applicable*).
//!
//! # The data lives in a file
//!
//! `data/jgma_116_02.csv`, one row per cell, so it can be diffed against the
//! standard rather than read out of Rust syntax. Its header names the source
//! transcribed. Each band is stored as printed, in interval notation: every
//! band is closed above (以下) and closed (以上) or open (をこえ) below as its
//! label says, so a gear on a printed edge reads the row that prints it.
//!
//! The tests check the file without the source: row counts per grade, every
//! value a preferred number, monotone in grade within a band, total above
//! tooth-to-tooth, adjacent bands meeting at one held edge, and the bands
//! against the printed labels typed a second time.
//!
//! **The standard is not in this repository.** It is copyrighted, and shipping
//! it is a different act from transcribing a table of numbers out of it — the
//! same distinction `docs/rationale.md` draws about ISO's values.

use std::sync::OnceLock;

/// Which of the standard's two grade scales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scale {
    /// Grades 0–6, modules 0.2–1.6.
    Fine,
    /// Grades 4–12, modules 1–10.
    Standard,
}

impl Scale {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fine => "fine",
            Self::Standard => "standard",
        }
    }
}

/// A tolerance class: a scale together with a grade number on that scale.
///
/// The scale is part of the identity precisely because grade numbers are not
/// comparable across scales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Class {
    pub scale: Scale,
    pub grade: u8,
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.scale {
            Scale::Fine => "Fine",
            Scale::Standard => "Standard",
        };
        write!(f, "{name} {}", self.grade)
    }
}

/// Allowable composite errors, micrometres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompositeError {
    /// 両歯面1ピッチかみあい誤差 — double-flank single-pitch (tooth-to-tooth)
    /// composite error.
    pub tooth_to_tooth: f64,
    /// 両歯面全かみあい誤差 — double-flank total composite error.
    pub total: f64,
}

/// A band as the standard prints it: closed above (以下), and closed below
/// (以上) or open below (をこえ) as the printed label says. Every printed band
/// is closed above, so only the lower end carries a closure.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Band {
    lo: f64,
    lo_closed: bool,
    hi: f64,
}

impl Band {
    /// `[lo~hi]` or `(lo~hi]`, as the data file writes a printed band.
    fn parse(s: &str) -> Option<Self> {
        let lo_closed = match s.get(..1)? {
            "[" => true,
            "(" => false,
            _ => return None,
        };
        let (lo, hi) = s.get(1..)?.strip_suffix(']')?.split_once('~')?;
        let (lo, hi) = (lo.parse().ok()?, hi.parse().ok()?);
        (lo < hi).then_some(Self { lo, lo_closed, hi })
    }

    fn contains(&self, v: f64) -> bool {
        (v > self.lo || self.lo_closed && v == self.lo) && v <= self.hi
    }
}

#[derive(Clone, Copy, Debug)]
struct Row {
    scale: Scale,
    grade: u8,
    module: Band,
    diameter: Band,
    error: CompositeError,
}

/// One data line, `scale,grade,module,diameter,tooth_to_tooth,total`, or
/// `None` if it is not one.
fn parse_row(line: &str) -> Option<Row> {
    let f: Vec<&str> = line.split(',').collect();
    let [scale, grade, module, diameter, t2t, total] = f.as_slice() else {
        return None;
    };
    Some(Row {
        scale: match *scale {
            "fine" => Scale::Fine,
            "standard" => Scale::Standard,
            _ => return None,
        },
        grade: grade.parse().ok()?,
        module: Band::parse(module)?,
        diameter: Band::parse(diameter)?,
        error: CompositeError {
            tooth_to_tooth: t2t.parse().ok()?,
            total: total.parse().ok()?,
        },
    })
}

const TABLE_CSV: &str = include_str!("../data/jgma_116_02.csv");

/// The data lines: every line but comments, the header and blanks.
fn data_lines() -> impl Iterator<Item = &'static str> {
    TABLE_CSV
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("scale,") && !l.trim().is_empty())
}

fn table() -> &'static [Row] {
    static TABLE: OnceLock<Vec<Row>> = OnceLock::new();
    TABLE.get_or_init(|| data_lines().filter_map(parse_row).collect())
}

/// Allowable composite errors for a class at a given module and pitch diameter.
///
/// Returns `None` when the standard has no entry — an out-of-range module or
/// diameter, or a grade that does not exist on that scale. That is a real
/// answer, not a failure: extrapolating a tolerance table is not a thing one
/// does.
#[must_use]
pub fn lookup(class: Class, module: f64, pitch_diameter: f64) -> Option<CompositeError> {
    table()
        .iter()
        .find(|r| {
            r.scale == class.scale
                && r.grade == class.grade
                && r.module.contains(module)
                && r.diameter.contains(pitch_diameter)
        })
        .map(|r| r.error)
}

/// Every class the standard actually covers for this gear, in display order.
///
/// This is what the UI's dropdown should offer. A fixed 0–12 list would present
/// classes that have no data — the specification's default of grade 3, for
/// instance, exists only below module 1.6.
#[must_use]
pub fn available_classes(module: f64, pitch_diameter: f64) -> Vec<Class> {
    let mut v: Vec<Class> = table()
        .iter()
        .filter(|r| r.module.contains(module) && r.diameter.contains(pitch_diameter))
        .map(|r| Class {
            scale: r.scale,
            grade: r.grade,
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The class to select when the user has not chosen: **fine scale first, then
/// lowest grade**.
///
/// Decided on scale and grade ordering alone, deliberately *not* on which entry
/// yields the smaller error. That keeps the default predictable and independent
/// of the table's contents, which is what lets it survive other standards being
/// added later.
#[must_use]
pub fn default_class(module: f64, pitch_diameter: f64) -> Option<Class> {
    available_classes(module, pitch_diameter).into_iter().next()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn spot_values_match_the_printed_tables() {
        // fine, grade 0, module 0.2-0.6, smallest diameter band: 5 / 13
        let e = lookup(
            Class {
                scale: Scale::Fine,
                grade: 0,
            },
            0.3,
            2.0,
        )
        .unwrap();
        assert_eq!(
            e,
            CompositeError {
                tooth_to_tooth: 5.0,
                total: 13.0
            }
        );

        // fine, grade 6, module (1, 1.6], largest diameter band: 60 / 200
        let e = lookup(
            Class {
                scale: Scale::Fine,
                grade: 6,
            },
            1.2,
            300.0,
        )
        .unwrap();
        assert_eq!(
            e,
            CompositeError {
                tooth_to_tooth: 60.0,
                total: 200.0
            }
        );

        // standard, grade 12, module 6.3-10, largest diameter band: 160 / 500
        let e = lookup(
            Class {
                scale: Scale::Standard,
                grade: 12,
            },
            8.0,
            3000.0,
        )
        .unwrap();
        assert_eq!(
            e,
            CompositeError {
                tooth_to_tooth: 160.0,
                total: 500.0
            }
        );

        // standard, grade 4, module 1-3.5, smallest band: 7 / 20 — the value
        // that is three times tighter than fine grade 4
        let e = lookup(
            Class {
                scale: Scale::Standard,
                grade: 4,
            },
            1.2,
            12.0,
        )
        .unwrap();
        assert_eq!(
            e,
            CompositeError {
                tooth_to_tooth: 7.0,
                total: 20.0
            }
        );
    }

    #[test]
    fn the_two_scales_are_not_interchangeable() {
        let (m, d) = (1.2, 12.0);
        let fine3 = lookup(
            Class {
                scale: Scale::Fine,
                grade: 3,
            },
            m,
            d,
        )
        .unwrap();
        let fine4 = lookup(
            Class {
                scale: Scale::Fine,
                grade: 4,
            },
            m,
            d,
        )
        .unwrap();
        let std4 = lookup(
            Class {
                scale: Scale::Standard,
                grade: 4,
            },
            m,
            d,
        )
        .unwrap();

        // Same grade number, three times the tolerance.
        assert!(std4.tooth_to_tooth < fine4.tooth_to_tooth / 2.0);

        // And the reason no merging rule works: taking the smaller value at each
        // grade makes the ladder DROP from grade 3 to grade 4.
        assert!(
            std4.tooth_to_tooth < fine3.tooth_to_tooth,
            "merged ladder would not be non-monotonic, so this argument needs revisiting"
        );
    }

    #[test]
    fn out_of_range_returns_nothing_rather_than_extrapolating() {
        // module above every band
        assert!(lookup(
            Class {
                scale: Scale::Standard,
                grade: 6
            },
            25.0,
            100.0
        )
        .is_none());
        // diameter below the fine table's smallest band
        assert!(lookup(
            Class {
                scale: Scale::Fine,
                grade: 0
            },
            0.3,
            1.0
        )
        .is_none());
        // grade that does not exist on that scale
        assert!(lookup(
            Class {
                scale: Scale::Fine,
                grade: 9
            },
            0.3,
            5.0
        )
        .is_none());
        assert!(lookup(
            Class {
                scale: Scale::Standard,
                grade: 0
            },
            2.0,
            100.0
        )
        .is_none());
    }

    #[test]
    fn available_classes_track_the_module() {
        // A fine-pitch gear: only the fine scale.
        let c = available_classes(0.4, 5.0);
        assert!(c.iter().all(|c| c.scale == Scale::Fine));
        assert_eq!(c.first().unwrap().grade, 0);

        // Module 2: only the standard scale, which starts at grade 4. This is
        // why the specification's fixed default of grade 3 cannot be used.
        let c = available_classes(2.0, 30.0);
        assert!(c.iter().all(|c| c.scale == Scale::Standard));
        assert_eq!(c.first().unwrap().grade, 4);
        assert!(!c.contains(&Class {
            scale: Scale::Standard,
            grade: 3
        }));

        // The overlap: both scales offered, user picks.
        let c = available_classes(1.2, 50.0);
        assert!(c.iter().any(|c| c.scale == Scale::Fine));
        assert!(c.iter().any(|c| c.scale == Scale::Standard));
    }

    #[test]
    fn default_is_fine_scale_then_lowest_grade() {
        // overlap region: fine wins, at its lowest grade
        assert_eq!(
            default_class(1.2, 50.0),
            Some(Class {
                scale: Scale::Fine,
                grade: 0
            })
        );
        // no fine coverage: falls to the standard scale's lowest
        assert_eq!(
            default_class(4.0, 200.0),
            Some(Class {
                scale: Scale::Standard,
                grade: 4
            })
        );
        // nothing at all
        assert_eq!(default_class(50.0, 5000.0), None);
    }

    /// **Adjacent bands meet at one edge, which exactly one of them holds.**
    /// A gap leaves a gear with no tolerance (a 12.00 mm gear once had none);
    /// an overlap gives it two.
    #[test]
    fn adjacent_bands_share_an_edge_that_one_of_them_holds() {
        let meet = |what: &str, bands: &mut Vec<Band>| {
            bands.sort_by(|a, b| a.lo.total_cmp(&b.lo));
            bands.dedup();
            for w in bands.windows(2) {
                assert!(
                    w[1].lo == w[0].hi && !w[1].lo_closed,
                    "{what}: {:?} then {:?}",
                    w[0],
                    w[1]
                );
            }
        };
        for scale in [Scale::Fine, Scale::Standard] {
            for grade in 0..=12u8 {
                let rows: Vec<&Row> = table()
                    .iter()
                    .filter(|r| r.scale == scale && r.grade == grade)
                    .collect();
                let mut modules: Vec<Band> = rows.iter().map(|r| r.module).collect();
                meet(&format!("{scale:?} {grade} modules"), &mut modules);
                for m in modules {
                    let mut diameters: Vec<Band> = rows
                        .iter()
                        .filter(|r| r.module == m)
                        .map(|r| r.diameter)
                        .collect();
                    meet(&format!("{scale:?} {grade} {m:?}"), &mut diameters);
                }
            }
        }
        for scale in [Scale::Fine, Scale::Standard] {
            let g = if scale == Scale::Fine { 0 } else { 4 };
            assert!(
                lookup(Class { scale, grade: g }, 1.2, 12.0).is_some(),
                "{scale:?}: a 12.00 mm gear must have a tolerance"
            );
        }
    }

    /// **The data file's bands, against the labels as printed, typed a second
    /// time.** 以上 is closed below, をこえ open below, 以下 closed above. The
    /// fine table's labels are the Japanese reproduction's, seen printed. The
    /// standard scale's were **not** seen printed: they are written here as
    /// ISO 1328:1975 prints the same scheme, an assumption recorded in
    /// `docs/state.md`. A band edge or its closure mistyped in either copy
    /// fails here.
    #[test]
    fn the_bands_are_the_printed_labels() {
        fn label(s: &str) -> Band {
            let (lo, lo_closed, rest) = if let Some((lo, rest)) = s.split_once("以上") {
                (lo.parse().unwrap(), true, rest)
            } else if let Some((lo, rest)) = s.split_once("をこえ") {
                (lo.parse().unwrap(), false, rest)
            } else {
                (0.0, true, s)
            };
            let hi = rest.strip_suffix("以下").unwrap().parse().unwrap();
            Band { lo, lo_closed, hi }
        }
        const FINE_D: [&str; 8] = [
            "1.5以上3以下",
            "3をこえ6以下",
            "6をこえ12以下",
            "12をこえ25以下",
            "25をこえ50以下",
            "50をこえ100以下",
            "100をこえ200以下",
            "200をこえ400以下",
        ];
        const STANDARD_D: [&str; 6] = [
            "125以下",
            "125をこえ400以下",
            "400をこえ800以下",
            "800をこえ1600以下",
            "1600をこえ2500以下",
            "2500をこえ4000以下",
        ];
        let printed: &[(Scale, &str, &[&str])] = &[
            (Scale::Fine, "0.2以上0.6以下", &FINE_D),
            (Scale::Fine, "0.6をこえ1以下", &FINE_D[1..]),
            (Scale::Fine, "1をこえ1.6以下", &FINE_D[2..]),
            (Scale::Standard, "1以上3.5以下", &STANDARD_D),
            (Scale::Standard, "3.5をこえ6.3以下", &STANDARD_D),
            (Scale::Standard, "6.3をこえ10以下", &STANDARD_D),
        ];
        let mut want: Vec<(Scale, Band, Band)> = Vec::new();
        for &(scale, m, ds) in printed {
            for d in ds {
                want.push((scale, label(m), label(d)));
            }
        }
        let grades = |scale| match scale {
            Scale::Fine => 0..=6u8,
            Scale::Standard => 4..=12u8,
        };
        for scale in [Scale::Fine, Scale::Standard] {
            for grade in grades(scale) {
                let mut got: Vec<(Scale, Band, Band)> = table()
                    .iter()
                    .filter(|r| r.scale == scale && r.grade == grade)
                    .map(|r| (r.scale, r.module, r.diameter))
                    .collect();
                let mut want: Vec<(Scale, Band, Band)> =
                    want.iter().filter(|w| w.0 == scale).copied().collect();
                let key = |x: &(Scale, Band, Band)| (x.1.lo, x.2.lo);
                got.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap());
                want.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap());
                assert_eq!(got, want, "{scale:?} grade {grade}");
            }
        }
    }

    /// **Some on the closed hull of what is printed, None just outside it**, for
    /// every class: at each band's edges and middle, a hair inside a held
    /// edge reads the row and a hair past an edge no neighbour holds reads none.
    #[test]
    fn a_lookup_is_some_exactly_where_a_band_is_printed() {
        let hair = 1e-9;
        let held = |scale: Scale, grade: u8, m: f64, d: f64| {
            table()
                .iter()
                .filter(|r| {
                    r.scale == scale
                        && r.grade == grade
                        && r.module.contains(m)
                        && r.diameter.contains(d)
                })
                .count()
        };
        for r in table() {
            let class = Class {
                scale: r.scale,
                grade: r.grade,
            };
            let mid = |b: &Band| 0.5 * (b.lo + b.hi);
            let (m, d) = (mid(&r.module), mid(&r.diameter));
            assert_eq!(lookup(class, m, d), Some(r.error), "{class} {r:?}");
            // Every point is held by at most one row.
            for (mm, dd) in [
                (r.module.hi, d),
                (r.module.lo, d),
                (m, r.diameter.hi),
                (m, r.diameter.lo),
            ] {
                assert!(held(r.scale, r.grade, mm, dd) <= 1, "{class} m={mm} d={dd}");
            }
            // An edge the row holds reads the row; one it does not reads
            // whatever holds it, and a hair inward reads the row.
            for (mm, dd, holds) in [
                (r.module.hi, d, true),
                (r.module.lo, d, r.module.lo_closed),
                (m, r.diameter.hi, true),
                (m, r.diameter.lo, r.diameter.lo_closed),
            ] {
                if holds {
                    assert_eq!(
                        lookup(class, mm, dd),
                        Some(r.error),
                        "{class} m={mm} d={dd}"
                    );
                }
            }
            assert_eq!(lookup(class, r.module.lo + hair, d), Some(r.error));
            assert_eq!(lookup(class, m, r.diameter.lo + hair), Some(r.error));
            // Past an edge: some row of this class holds it, or none does and
            // the lookup is None.
            for (mm, dd) in [
                (r.module.hi + hair, d),
                (r.module.lo - hair, d),
                (m, r.diameter.hi + hair),
                (m, r.diameter.lo - hair),
            ] {
                assert_eq!(
                    lookup(class, mm, dd).is_some(),
                    held(r.scale, r.grade, mm, dd) == 1,
                    "{class} m={mm} d={dd}"
                );
            }
        }
        // The hull's corners, as printed.
        let fine0 = Class {
            scale: Scale::Fine,
            grade: 0,
        };
        let std4 = Class {
            scale: Scale::Standard,
            grade: 4,
        };
        assert!(lookup(fine0, 0.2, 1.5).is_some() && lookup(fine0, 1.6, 400.0).is_some());
        assert!(lookup(fine0, 0.2 - hair, 1.5).is_none());
        assert!(lookup(fine0, 0.2, 1.5 - hair).is_none());
        assert!(lookup(fine0, 1.6 + hair, 400.0).is_none());
        assert!(lookup(fine0, 1.6, 400.0 + hair).is_none());
        assert!(lookup(std4, 1.0, 1.0).is_some() && lookup(std4, 10.0, 4000.0).is_some());
        assert!(lookup(std4, 1.0 - hair, 100.0).is_none());
        assert!(lookup(std4, 10.0 + hair, 100.0).is_none());
        assert!(lookup(std4, 10.0, 4000.0 + hair).is_none());
    }

    /// A line the parser cannot read would be dropped from the table without
    /// a word; this names it.
    #[test]
    fn every_data_line_parses() {
        for (i, line) in data_lines().enumerate() {
            assert!(parse_row(line).is_some(), "data line {i}: {line:?}");
        }
    }

    /// **A printed edge belongs to the band that prints it closed.** The fine
    /// table's blocks read `m0.2以上0.6以下`, `m0.6をこえ1以下`, `m1をこえ1.6以下`
    /// and its diameter bands `1.5以上3以下`, `3をこえ6以下`, …: every band is
    /// closed above, and only the m0.2–0.6 block's first band is closed below.
    #[test]
    fn every_printed_edge_reads_the_row_that_prints_it() {
        let fine = |g| Class {
            scale: Scale::Fine,
            grade: g,
        };
        let standard = |g| Class {
            scale: Scale::Standard,
            grade: g,
        };
        let e = |a, b| {
            Some(CompositeError {
                tooth_to_tooth: a,
                total: b,
            })
        };
        let cases: &[(Class, f64, f64, Option<CompositeError>)] = &[
            // m = 1 is the (0.6, 1] block, not (1, 1.6]
            (fine(0), 1.0, 9.0, e(6.0, 18.0)),
            (fine(0), 1.0, 20.0, e(6.7, 18.0)),
            // m = 0.6 is the [0.2, 0.6] block
            (fine(0), 0.6, 20.0, e(6.3, 18.0)),
            // m = 1.6 closes the (1, 1.6] block
            (fine(0), 1.6, 20.0, e(7.1, 20.0)),
            (fine(0), 1.600_001, 20.0, None),
            (fine(0), 0.2, 2.0, e(5.0, 13.0)),
            (fine(0), 0.199_999, 2.0, None),
            // d = 1.5 opens the first band closed; d = 3 closes it
            (fine(0), 0.3, 1.5, e(5.0, 13.0)),
            (fine(0), 0.3, 1.499_999, None),
            (fine(0), 0.3, 3.0, e(5.0, 13.0)),
            (fine(0), 0.3, 3.005, e(5.3, 16.0)),
            (fine(0), 0.3, 6.005, e(5.6, 16.0)),
            (fine(0), 0.3, 400.0, e(8.0, 24.0)),
            (fine(0), 0.3, 400.005, None),
            // the (0.6, 1] block opens at 3, not at or below it
            (fine(0), 0.8, 3.0, None),
            (fine(0), 0.8, 3.000_001, e(5.6, 16.0)),
            (fine(0), 1.2, 6.0, None),
            (fine(0), 1.2, 6.000_001, e(6.3, 18.0)),
            // the standard scale, read (lo, hi] as ISO 1328:1975 prints it
            (standard(4), 10.0, 100.0, e(10.0, 28.0)),
            (standard(4), 10.000_001, 100.0, None),
            (standard(4), 1.0, 100.0, e(7.0, 20.0)),
            (standard(4), 0.999_999, 100.0, None),
            (standard(4), 3.5, 100.0, e(7.0, 20.0)),
            (standard(4), 6.3, 100.0, e(9.0, 25.0)),
            (standard(4), 2.0, 125.0, e(7.0, 20.0)),
            (standard(4), 2.0, 125.000_001, e(8.0, 22.0)),
            (standard(4), 2.0, 4000.0, e(13.0, 36.0)),
            (standard(4), 2.0, 4_000.000_001, None),
        ];
        let mut wrong = Vec::new();
        for &(class, m, d, want) in cases {
            let got = lookup(class, m, d);
            if got != want {
                wrong.push(format!("{class} m={m} d={d}: {got:?}, printed {want:?}"));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert_eq!(default_class(10.0, 400.0), Some(standard(4)));
        assert!(available_classes(1.6, 30.0).contains(&fine(0)));
    }

    // ---- transcription checks on the data file ---------------------- //

    #[test]
    fn every_grade_has_the_expected_number_of_cells() {
        for grade in 0..=6u8 {
            let n = table()
                .iter()
                .filter(|r| r.scale == Scale::Fine && r.grade == grade)
                .count();
            assert_eq!(n, 21, "fine grade {grade} has {n} cells, expected 8+7+6");
        }
        for grade in 4..=12u8 {
            let n = table()
                .iter()
                .filter(|r| r.scale == Scale::Standard && r.grade == grade)
                .count();
            assert_eq!(n, 18, "standard grade {grade} has {n} cells, expected 3x6");
        }
        assert_eq!(table().len(), 7 * 21 + 9 * 18);
    }

    /// Every value in the standard is a preferred number. A digit transposed
    /// during transcription almost certainly is not, which is what makes this
    /// worth asserting.
    #[test]
    fn every_value_is_a_preferred_number() {
        const ALLOWED: &[f64] = &[
            5.0, 5.3, 5.6, 6.0, 6.3, 6.7, 7.0, 7.1, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 24.0, 25.0, 26.0, 28.0,
            30.0, 32.0, 34.0, 36.0, 38.0, 40.0, 42.0, 45.0, 48.0, 50.0, 53.0, 56.0, 60.0, 63.0,
            67.0, 71.0, 75.0, 80.0, 85.0, 90.0, 95.0, 100.0, 112.0, 125.0, 140.0, 160.0, 180.0,
            200.0, 224.0, 250.0, 280.0, 315.0, 355.0, 400.0, 450.0, 500.0,
        ];
        // 7.0 is the standard's own rounding of 7.1 and is the only entry that
        // is not strictly in the R40 series.
        let ok = |v: f64| ALLOWED.iter().any(|a| (a - v).abs() < 1e-9);
        for r in table() {
            assert!(
                ok(r.error.tooth_to_tooth),
                "{:?}: {}",
                r.scale,
                r.error.tooth_to_tooth
            );
            assert!(ok(r.error.total), "{:?}: {}", r.scale, r.error.total);
        }
    }

    /// Within one scale and band, a coarser grade must never be tighter. A
    /// misaligned column shows up here immediately.
    #[test]
    fn tolerances_grow_with_grade_within_every_band() {
        for scale in [Scale::Fine, Scale::Standard] {
            let bands: Vec<(f64, f64)> = {
                let mut b: Vec<(f64, f64)> = table()
                    .iter()
                    .filter(|r| r.scale == scale)
                    .map(|r| (r.module.lo, r.diameter.lo))
                    .collect();
                b.sort_by(|a, b| a.partial_cmp(b).unwrap());
                b.dedup();
                b
            };
            for (m, d) in bands {
                let mut rows: Vec<&Row> = table()
                    .iter()
                    .filter(|r| r.scale == scale && r.module.lo == m && r.diameter.lo == d)
                    .collect();
                rows.sort_by_key(|r| r.grade);
                for w in rows.windows(2) {
                    assert!(
                        w[1].error.tooth_to_tooth >= w[0].error.tooth_to_tooth
                            && w[1].error.total >= w[0].error.total,
                        "{scale:?} m>={m} d>={d}: grade {} is tighter than grade {}",
                        w[1].grade,
                        w[0].grade
                    );
                }
            }
        }
    }

    /// Total composite error covers a whole revolution and so can never be
    /// smaller than the single-tooth figure.
    #[test]
    fn total_error_always_exceeds_tooth_to_tooth() {
        for r in table() {
            assert!(
                r.error.total > r.error.tooth_to_tooth,
                "{:?} grade {}: total {} <= tooth-to-tooth {}",
                r.scale,
                r.grade,
                r.error.total,
                r.error.tooth_to_tooth
            );
        }
    }
}
