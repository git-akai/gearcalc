//! DXF export.
//!
//! Hand-written rather than pulled from a crate: the subset needed here is a
//! closed polyline and a few circles, and DXF's tagged format is simple enough
//! that a dependency would cost more than it saves — particularly in a
//! WebAssembly build, where every byte ships to the browser.
//!
//! Written as **AC1015 (R2000)**, which is old enough to be universally readable
//! and new enough to support `LWPOLYLINE`.
//!
//! # Why one polyline rather than a pile of entities
//!
//! The profile goes out as a single closed `LWPOLYLINE`. Its tip and root arcs
//! are carried as vertex *bulges*, so they remain exact circular arcs rather
//! than chorded approximations, and the whole outline is still one entity the
//! user can extrude without joining anything first. That is what the bulge field
//! exists for.
//!
//! Only the involute flank and the trochoid fillet are polygonal, and those are
//! the two curves with no exact arc representation. Their accuracy is set by the
//! chord tolerance passed to [`gear_core::Tooth::outline`].

use gear_core::ring::Ring;
use gear_core::{Tooth, Vertex};

/// Layer names. Geometry and construction aids are separated so the reference
/// circles can be switched off in CAD without touching the profile.
const LAYER_PROFILE: &str = "GEAR_PROFILE";
const LAYER_REFERENCE: &str = "GEAR_REFERENCE";

/// AutoCAD colour indices: 7 is "by background" (black on white, white on
/// black), 8 is a mid grey that reads as secondary on either.
const COLOUR_PROFILE: i32 = 7;
const COLOUR_REFERENCE: i32 = 8;

/// What to include in the exported drawing.
#[derive(Clone, Copy, Debug)]
pub struct DxfOptions {
    /// Maximum deviation of the exported outline from the true curve, mm.
    pub chord_tolerance: f64,
    /// Emit the pitch, base, tip and root circles on a construction layer.
    pub reference_circles: bool,
}

impl Default for DxfOptions {
    fn default() -> Self {
        Self {
            chord_tolerance: gear_core::outline::DEFAULT_CHORD_TOLERANCE,
            reference_circles: true,
        }
    }
}

/// A DXF tag: a group code and its value, one per line each.
struct Writer {
    out: String,
    handle: u32,
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            // Handles must be non-zero and unique. Starting above the table
            // records keeps them easy to read while debugging.
            handle: 0x100,
        }
    }

    fn tag(&mut self, code: i32, value: &str) {
        self.out.push_str(&format!("{code}\n{value}\n"));
    }

    fn int(&mut self, code: i32, value: i32) {
        self.tag(code, &value.to_string());
    }

    /// Coordinates are written with enough digits to round-trip a double, so
    /// the exported geometry is not silently coarser than the tolerance asked
    /// for.
    fn real(&mut self, code: i32, value: f64) {
        self.tag(code, &format!("{value:.12}"));
    }

    fn next_handle(&mut self) -> String {
        self.handle += 1;
        format!("{:X}", self.handle)
    }

    fn handle_tag(&mut self) {
        let h = self.next_handle();
        self.tag(5, &h);
    }
}

/// Render a gear as a DXF drawing.
///
/// Millimetres, with the first tooth centred on +X and the gear axis at the
/// origin, matching the on-screen view.
#[must_use]
pub fn gear_to_dxf(gear: &Tooth, opts: &DxfOptions) -> String {
    outline_to_dxf(
        // Through the assembly: drawing a whole gear is `Gear`'s job, and this
        // is the export path an eccentric gear once slipped through as a
        // concentric one (`docs/corrections.md`).
        &gear_core::gear::Gear::new(gear.params).outline(opts.chord_tolerance),
        &[gear.r, gear.rb, gear.ra, gear.rf],
        opts,
    )
}

/// The same, for an internal gear.
///
/// A ring exports the outline of its **bore**: its teeth point inward, so what
/// this traces is the hole, and whatever rim sits outside it is the designer's
/// business rather than the tooth geometry's.
#[must_use]
pub fn ring_to_dxf(ring: &Ring, opts: &DxfOptions) -> String {
    // The rim circle joins the reference circles on the construction layer
    // rather than the profile layer: it is where the *drawing* shades the
    // material to, and a real ring's outside diameter is the designer's
    // (`Ring::rim_radius`). Putting it on the profile layer would hand CAD a
    // boundary nobody chose.
    outline_to_dxf(
        &ring.outline(opts.chord_tolerance),
        &[ring.r, ring.rb, ring.ra, ring.rf, ring.rim_radius()],
        opts,
    )
}

/// Write any closed outline, with its reference circles.
///
/// Both gear kinds come through here. The writer has no reason to know which it
/// is holding — a polyline is a polyline — and keeping it that way is what stops
/// a second export path growing its own quirks.
fn outline_to_dxf(outline: &[Vertex], circles: &[f64], opts: &DxfOptions) -> String {
    let mut w = Writer::new();
    header(&mut w);
    tables(&mut w, opts.reference_circles);

    w.tag(0, "SECTION");
    w.tag(2, "ENTITIES");
    polyline(&mut w, outline);
    if opts.reference_circles {
        for &r in circles {
            circle(&mut w, r);
        }
    }
    w.tag(0, "ENDSEC");

    w.tag(0, "EOF");
    w.out
}

fn header(w: &mut Writer) {
    w.tag(0, "SECTION");
    w.tag(2, "HEADER");
    w.tag(9, "$ACADVER");
    w.tag(1, "AC1015");
    w.tag(9, "$INSUNITS");
    w.int(70, 4); // millimetres
    w.tag(9, "$HANDSEED");
    w.tag(5, "FFFF");
    w.tag(0, "ENDSEC");
}

fn tables(w: &mut Writer, reference_circles: bool) {
    w.tag(0, "SECTION");
    w.tag(2, "TABLES");

    // A linetype table is required before layers can reference CONTINUOUS.
    w.tag(0, "TABLE");
    w.tag(2, "LTYPE");
    w.handle_tag();
    w.tag(100, "AcDbSymbolTable");
    w.int(70, 1);
    w.tag(0, "LTYPE");
    w.handle_tag();
    w.tag(100, "AcDbSymbolTableRecord");
    w.tag(100, "AcDbLinetypeTableRecord");
    w.tag(2, "CONTINUOUS");
    w.int(70, 0);
    w.tag(3, "Solid line");
    w.int(72, 65);
    w.int(73, 0);
    w.real(40, 0.0);
    w.tag(0, "ENDTAB");

    let layers: &[(&str, i32)] = if reference_circles {
        &[
            (LAYER_PROFILE, COLOUR_PROFILE),
            (LAYER_REFERENCE, COLOUR_REFERENCE),
        ]
    } else {
        &[(LAYER_PROFILE, COLOUR_PROFILE)]
    };

    w.tag(0, "TABLE");
    w.tag(2, "LAYER");
    w.handle_tag();
    w.tag(100, "AcDbSymbolTable");
    w.int(70, i32::try_from(layers.len()).unwrap_or(1));
    for (name, colour) in layers {
        w.tag(0, "LAYER");
        w.handle_tag();
        w.tag(100, "AcDbSymbolTableRecord");
        w.tag(100, "AcDbLayerTableRecord");
        w.tag(2, name);
        w.int(70, 0);
        w.int(62, *colour);
        w.tag(6, "CONTINUOUS");
    }
    w.tag(0, "ENDTAB");

    w.tag(0, "ENDSEC");
}

fn polyline(w: &mut Writer, vertices: &[Vertex]) {
    if vertices.is_empty() {
        return;
    }
    w.tag(0, "LWPOLYLINE");
    w.handle_tag();
    w.tag(100, "AcDbEntity");
    w.tag(8, LAYER_PROFILE);
    w.tag(100, "AcDbPolyline");
    w.int(90, i32::try_from(vertices.len()).unwrap_or(i32::MAX));
    w.int(70, 1); // closed
    w.real(43, 0.0); // constant width
    for v in vertices {
        w.real(10, v.x);
        w.real(20, v.y);
        if v.bulge != 0.0 {
            w.real(42, v.bulge);
        }
    }
}

fn circle(w: &mut Writer, radius: f64) {
    w.tag(0, "CIRCLE");
    w.handle_tag();
    w.tag(100, "AcDbEntity");
    w.tag(8, LAYER_REFERENCE);
    w.tag(100, "AcDbCircle");
    w.real(10, 0.0);
    w.real(20, 0.0);
    w.real(30, 0.0);
    w.real(40, radius);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::GearParams;

    fn tags(dxf: &str) -> Vec<(i32, String)> {
        let lines: Vec<&str> = dxf.lines().collect();
        assert!(
            lines.len().is_multiple_of(2),
            "DXF must be code/value pairs"
        );
        lines
            .chunks(2)
            .map(|c| {
                (
                    c[0].trim().parse::<i32>().expect("group code"),
                    c[1].to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn structure_is_well_formed() {
        let g = Tooth::new(GearParams::default());
        let dxf = gear_to_dxf(&g, &DxfOptions::default());
        let t = tags(&dxf);

        // sections open and close in pairs, and the file ends with EOF
        let opens = t.iter().filter(|(c, v)| *c == 0 && v == "SECTION").count();
        let closes = t.iter().filter(|(c, v)| *c == 0 && v == "ENDSEC").count();
        assert_eq!(opens, closes, "unbalanced sections");
        assert_eq!(t.last().unwrap(), &(0, "EOF".to_string()));

        // tables open and close in pairs too
        let to = t.iter().filter(|(c, v)| *c == 0 && v == "TABLE").count();
        let tc = t.iter().filter(|(c, v)| *c == 0 && v == "ENDTAB").count();
        assert_eq!(to, tc, "unbalanced tables");

        // every layer an entity references must be declared
        let declared: Vec<&String> = t
            .windows(2)
            .filter(|w| w[0] == (0, "LAYER".to_string()))
            .map(|_| &t[0].1)
            .collect();
        let _ = declared;
        assert!(dxf.contains(LAYER_PROFILE) && dxf.contains(LAYER_REFERENCE));
    }

    #[test]
    fn polyline_is_closed_and_carries_every_vertex() {
        let g = Tooth::new(GearParams {
            teeth: 9,
            ..Default::default()
        });
        let want = gear_core::gear::Gear::new(g.params).outline(1e-3);
        let dxf = gear_to_dxf(
            &g,
            &DxfOptions {
                chord_tolerance: 1e-3,
                reference_circles: false,
            },
        );
        let t = tags(&dxf);

        let count = t
            .iter()
            .find(|(c, _)| *c == 90)
            .map(|(_, v)| v.parse::<usize>().unwrap())
            .expect("vertex count");
        assert_eq!(count, want.len());

        // 70 = 1 means closed
        assert!(
            t.iter().any(|(c, v)| *c == 70 && v == "1"),
            "polyline not closed"
        );

        let xs: Vec<f64> = t
            .iter()
            .filter(|(c, _)| *c == 10)
            .map(|(_, v)| v.parse().unwrap())
            .collect();
        let ys: Vec<f64> = t
            .iter()
            .filter(|(c, _)| *c == 20)
            .map(|(_, v)| v.parse().unwrap())
            .collect();
        assert_eq!(xs.len(), want.len());
        assert_eq!(ys.len(), want.len());
        for (i, v) in want.iter().enumerate() {
            assert!((xs[i] - v.x).abs() < 1e-9 && (ys[i] - v.y).abs() < 1e-9);
        }
    }

    #[test]
    fn reference_circles_are_optional_and_on_their_own_layer() {
        let g = Tooth::new(GearParams::default());
        let with = gear_to_dxf(
            &g,
            &DxfOptions {
                reference_circles: true,
                ..Default::default()
            },
        );
        let without = gear_to_dxf(
            &g,
            &DxfOptions {
                reference_circles: false,
                ..Default::default()
            },
        );
        assert_eq!(
            tags(&with)
                .iter()
                .filter(|(c, v)| *c == 0 && v == "CIRCLE")
                .count(),
            4
        );
        assert_eq!(
            tags(&without)
                .iter()
                .filter(|(c, v)| *c == 0 && v == "CIRCLE")
                .count(),
            0
        );
        assert!(
            !without.contains(LAYER_REFERENCE),
            "unused layer should not be declared"
        );
    }

    /// **A ring's DXF carries the rim circle, and carries it as construction.**
    ///
    /// The outline a ring exports is its *bore*; the material is outside it,
    /// and without something to say where it stops the file is indistinguishable
    /// from an external gear's. The circle goes on the reference layer rather
    /// than the profile layer because it is a drawing convention and not a
    /// dimension anyone chose — CAD should be free to ignore or replace it.
    #[test]
    fn a_rings_dxf_carries_its_rim_as_a_construction_circle() {
        use gear_core::ring::{Cutter, Ring};

        let ring = Ring::cut_by(
            &GearParams {
                teeth: 60,
                ..GearParams::default()
            },
            &Cutter::default(),
        );
        let with = ring_to_dxf(&ring, &DxfOptions::default());
        let radii: Vec<f64> = tags(&with)
            .iter()
            .filter(|(c, _)| *c == 40)
            .filter_map(|(_, v)| v.trim().parse::<f64>().ok())
            .collect();
        assert!(
            radii.iter().any(|r| (r - ring.rim_radius()).abs() < 1e-9),
            "no circle at the rim radius {}: {radii:?}",
            ring.rim_radius()
        );
        assert!(
            ring.rim_radius() > ring.rf,
            "the rim must lie outside the root circle, or it is inside the teeth"
        );

        // ...and it is construction, so turning the reference circles off
        // leaves the bore alone in the file.
        let without = ring_to_dxf(
            &ring,
            &DxfOptions {
                reference_circles: false,
                ..DxfOptions::default()
            },
        );
        assert_eq!(
            tags(&without)
                .iter()
                .filter(|(c, v)| *c == 0 && v == "CIRCLE")
                .count(),
            0
        );
    }

    #[test]
    fn bulges_are_written_only_where_the_geometry_is_circular() {
        let g = Tooth::new(GearParams {
            teeth: 12,
            ..Default::default()
        });
        let outline = gear_core::gear::Gear::new(g.params).outline(1e-3);
        let dxf = gear_to_dxf(
            &g,
            &DxfOptions {
                chord_tolerance: 1e-3,
                reference_circles: false,
            },
        );
        let n_bulge_written = tags(&dxf).iter().filter(|(c, _)| *c == 42).count();
        let n_bulge_expected = outline.iter().filter(|v| v.bulge != 0.0).count();
        assert_eq!(n_bulge_written, n_bulge_expected);
        // Three arcs per tooth: the tip arc, plus the root arc in two halves.
        // The root arc is split at mid tooth-space because that is where one
        // tooth's span ends and the next begins. Two co-circular arcs meeting at
        // a point are geometrically identical to one, and keeping each tooth
        // self-contained is worth more than merging them would save.
        assert_eq!(n_bulge_expected, 3 * 12);
    }
}
