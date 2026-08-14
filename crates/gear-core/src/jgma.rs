//! JGMA 116-02 (1983) gear meshing error tolerances.
//!
//! The standard is a **banded lookup table, not a formula**: find the module
//! band, find the pitch-diameter band, read two numbers in micrometres. Nothing
//! is interpolated, because the standard does not interpolate.
//!
//! # Two scales, not one
//!
//! The document contains two tables, and they are **independent grade scales**.
//! A grade number on one is not the same tolerance as the same number on the
//! other. At module 1.0–1.6 and a 12 mm pitch diameter, where both tables apply:
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
//! annotation supports this: the fine table's 1.0–1.6 column is marked 選用
//! (*optional*), while its finer columns are 適用 (*applicable*).
//!
//! # The data lives in a file
//!
//! `data/jgma_116_02.csv`, one row per cell, so it can be diffed against the PDF
//! rather than read out of Rust syntax. Three transcription checks run in the
//! test suite: row counts per grade, every value a preferred number, and
//! monotonicity in grade within a band. Those catch column misalignment, which
//! is the realistic failure mode when transcribing a scanned table — and which
//! the raw text extraction did in fact exhibit before the pages were checked as
//! images.

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
    /// 一齒嚙合誤差 — single-tooth (tooth-to-tooth) composite error.
    pub tooth_to_tooth: f64,
    /// 全齒嚙合誤差 — total composite error.
    pub total: f64,
}

#[derive(Clone, Copy, Debug)]
struct Row {
    scale: Scale,
    grade: u8,
    module: (f64, f64),
    diameter: (f64, f64),
    error: CompositeError,
}

const TABLE_CSV: &str = include_str!("../data/jgma_116_02.csv");

fn table() -> &'static [Row] {
    static TABLE: OnceLock<Vec<Row>> = OnceLock::new();
    TABLE.get_or_init(|| {
        TABLE_CSV
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("scale,") && !l.trim().is_empty())
            .filter_map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                if f.len() != 8 {
                    return None;
                }
                let n = |i: usize| f[i].parse::<f64>().ok();
                Some(Row {
                    scale: match f[0] {
                        "fine" => Scale::Fine,
                        "standard" => Scale::Standard,
                        _ => return None,
                    },
                    grade: f[1].parse().ok()?,
                    module: (n(2)?, n(3)?),
                    diameter: (n(4)?, n(5)?),
                    error: CompositeError {
                        tooth_to_tooth: n(6)?,
                        total: n(7)?,
                    },
                })
            })
            .collect()
    })
}

/// Bands are half-open: `[lo, hi)`.
fn in_band(v: f64, (lo, hi): (f64, f64)) -> bool {
    v >= lo && v < hi
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
                && in_band(module, r.module)
                && in_band(pitch_diameter, r.diameter)
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
        .filter(|r| in_band(module, r.module) && in_band(pitch_diameter, r.diameter))
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

        // fine, grade 6, module 1.0-1.6, largest diameter band: 60 / 200
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

    #[test]
    fn module_bands_are_half_open() {
        // A module exactly on a boundary belongs to the upper band.
        let low = lookup(
            Class {
                scale: Scale::Fine,
                grade: 0,
            },
            0.599,
            5.0,
        )
        .unwrap();
        let high = lookup(
            Class {
                scale: Scale::Fine,
                grade: 0,
            },
            0.6,
            5.0,
        )
        .unwrap();
        assert_ne!(low, high, "0.6 must fall in the 0.6-1.0 band, not 0.2-0.6");
    }

    /// Caught a real defect. The fine table prints inclusive diameter bands that
    /// step by 0.01 -- "6.01~12.00" then "12.01~25.00" -- so stored half-open as
    /// printed, a gear of exactly 12.00 mm falls in the gap between two rows and
    /// gets no tolerance at all. Bands are stored contiguously instead.
    #[test]
    fn coverage_is_contiguous_within_every_scale() {
        for scale in [Scale::Fine, Scale::Standard] {
            for grade in 0..=12u8 {
                let mut bands: Vec<(f64, f64)> = table()
                    .iter()
                    .filter(|r| r.scale == scale && r.grade == grade)
                    .map(|r| r.diameter)
                    .collect();
                if bands.is_empty() {
                    continue;
                }
                bands.sort_by(|a, b| a.partial_cmp(b).unwrap());
                bands.dedup();
                for w in bands.windows(2) {
                    assert!(
                        (w[1].0 - w[0].1).abs() < 1e-9,
                        "{scale:?} grade {grade}: gap between diameter bands \
                         ending {} and starting {}",
                        w[0].1,
                        w[1].0
                    );
                }
            }
        }
        // and the specific value that exposed it
        for scale in [Scale::Fine, Scale::Standard] {
            let g = if scale == Scale::Fine { 0 } else { 4 };
            assert!(
                lookup(Class { scale, grade: g }, 1.2, 12.0).is_some(),
                "{scale:?}: a 12.00 mm gear must have a tolerance"
            );
        }
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
                    .map(|r| (r.module.0, r.diameter.0))
                    .collect();
                b.sort_by(|a, b| a.partial_cmp(b).unwrap());
                b.dedup();
                b
            };
            for (m, d) in bands {
                let mut rows: Vec<&Row> = table()
                    .iter()
                    .filter(|r| {
                        r.scale == scale
                            && (r.module.0 - m).abs() < 1e-12
                            && (r.diameter.0 - d).abs() < 1e-12
                    })
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
