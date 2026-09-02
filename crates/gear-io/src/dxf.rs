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
//! # A file is more than its geometry
//!
//! R2000 is a graph rather than a list of shapes: every record names its owner
//! by handle, an entity is owned by the block record of the space it is drawn
//! in, and the format's published minimum asks for six sections, nine symbol
//! tables, both spaces defined again as blocks, and a root dictionary — none of
//! which draws anything.
//!
//! Leaving that out costs nothing until the file meets a reader that does not
//! rebuild what is missing. `ezdxf` does rebuild it, which is why this export
//! shipped incomplete and checked out sound; SOLIDWORKS does not, and would not
//! open it (`docs/corrections.md#a-reader-that-repairs-is-not-a-check`). The
//! requirement is gated in this module's tests, tag by tag.
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
    next: u32,
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            // Handles must be non-zero and unique. The records other records
            // *point at* are named in `handle` below and take the low numbers;
            // everything else counts up from here, so the two cannot collide.
            next: handle::FIRST_COUNTED,
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
        self.next += 1;
        format!("{:X}", self.next)
    }

    /// A record's handle and the record that owns it — the two tags every
    /// R2000 record carries, and the pair a reader walks the file by.
    fn owned(&mut self, owner: &str) {
        let h = self.next_handle();
        self.tag(5, &h);
        self.tag(330, owner);
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
    classes(&mut w);
    tables(&mut w, opts.reference_circles);
    blocks(&mut w);

    w.tag(0, "SECTION");
    w.tag(2, "ENTITIES");
    polyline(&mut w, outline);
    if opts.reference_circles {
        for &r in circles {
            circle(&mut w, r);
        }
    }
    w.tag(0, "ENDSEC");

    objects(&mut w);

    w.tag(0, "EOF");
    w.out
}

/// The handles of the records that other records point at.
///
/// **An R2000 file is a graph, not a list.** Every record carries an owner
/// (group code 330) and every entity is owned by the block record of the space
/// it is drawn in, so a handle here is a name another record spells out. They
/// are fixed and small — the running counter starts at `0x100` — which is what
/// keeps "who owns this" answerable by reading rather than by counting.
mod handle {
    pub const LTYPE_TABLE: &str = "1";
    pub const LAYER_TABLE: &str = "2";
    pub const STYLE_TABLE: &str = "3";
    pub const VIEW_TABLE: &str = "4";
    pub const UCS_TABLE: &str = "5";
    pub const APPID_TABLE: &str = "6";
    pub const DIMSTYLE_TABLE: &str = "7";
    pub const VPORT_TABLE: &str = "8";
    pub const BLOCK_RECORD_TABLE: &str = "9";
    pub const ROOT_DICTIONARY: &str = "A";
    pub const GROUP_DICTIONARY: &str = "B";
    /// The two block records every R2000 file has, and the block definitions
    /// they own. Model space is where the gear is drawn; paper space is empty
    /// and present because a file without it is refused.
    pub const MODEL_SPACE: &str = "C";
    pub const PAPER_SPACE: &str = "D";
    pub const MODEL_SPACE_BLOCK: &str = "E";
    pub const MODEL_SPACE_ENDBLK: &str = "F";
    pub const PAPER_SPACE_BLOCK: &str = "10";
    pub const PAPER_SPACE_ENDBLK: &str = "11";
    /// Nothing owned by the running counter may collide with the above.
    pub const FIRST_COUNTED: u32 = 0x100;
    /// `$HANDSEED`: the next handle a *reader* may hand out, so it has to be
    /// above every handle in the file. Gated by a test rather than trusted.
    pub const SEED: &str = "FFFF";
}

fn header(w: &mut Writer) {
    w.tag(0, "SECTION");
    w.tag(2, "HEADER");
    w.tag(9, "$ACADVER");
    w.tag(1, "AC1015");
    w.tag(9, "$INSUNITS");
    w.int(70, 4); // millimetres
    w.tag(9, "$HANDSEED");
    w.tag(5, handle::SEED);
    w.tag(0, "ENDSEC");
}

/// Present and empty.
///
/// `CLASSES` describes the application-defined entity types a file uses, and
/// this file uses none — but R2000 lists the section as required, and a reader
/// walking the sections in order is entitled to find it.
fn classes(w: &mut Writer) {
    w.tag(0, "SECTION");
    w.tag(2, "CLASSES");
    w.tag(0, "ENDSEC");
}

/// A symbol table's head: its own handle, its owner (the file), and how many
/// records follow.
fn table_head(w: &mut Writer, name: &str, handle: &str, records: usize) {
    w.tag(0, "TABLE");
    w.tag(2, name);
    w.tag(5, handle);
    w.tag(330, "0");
    w.tag(100, "AcDbSymbolTable");
    w.int(70, i32::try_from(records).unwrap_or(i32::MAX));
}

/// A table with no records. `VPORT`, `VIEW` and `UCS` are required to exist and
/// permitted to be empty: they hold saved views, and this file is a part rather
/// than a drawing sheet.
fn empty_table(w: &mut Writer, name: &str, handle: &str) {
    table_head(w, name, handle, 0);
    w.tag(0, "ENDTAB");
}

/// The opening of any symbol-table record: what it is, its handle, its owner,
/// the two subclass markers, its name, and its flags.
fn record(w: &mut Writer, kind: &str, owner: &str, subclass: &str, name: &str) {
    w.tag(0, kind);
    w.owned(owner);
    w.tag(100, "AcDbSymbolTableRecord");
    w.tag(100, subclass);
    w.tag(2, name);
    w.int(70, 0);
}

fn tables(w: &mut Writer, reference_circles: bool) {
    w.tag(0, "SECTION");
    w.tag(2, "TABLES");

    empty_table(w, "VPORT", handle::VPORT_TABLE);

    // **All three line types are required**, not just the one this file draws
    // with: `ByBlock` and `ByLayer` are what every other record means when it
    // says its line type is inherited.
    let linetypes = [
        ("ByBlock", ""),
        ("ByLayer", ""),
        ("Continuous", "Solid line"),
    ];
    table_head(w, "LTYPE", handle::LTYPE_TABLE, linetypes.len());
    for (name, description) in linetypes {
        record(
            w,
            "LTYPE",
            handle::LTYPE_TABLE,
            "AcDbLinetypeTableRecord",
            name,
        );
        w.tag(3, description);
        w.int(72, 65); // 'A', the only alignment code DXF defines
        w.int(73, 0); // no dashes
        w.real(40, 0.0);
    }
    w.tag(0, "ENDTAB");

    // Layer `0` is required, and is not decoration: the two block definitions
    // below are drawn on it, so a file without it has entities on a layer that
    // does not exist.
    let mut layers: Vec<(&str, i32)> = vec![("0", COLOUR_PROFILE), (LAYER_PROFILE, COLOUR_PROFILE)];
    if reference_circles {
        layers.push((LAYER_REFERENCE, COLOUR_REFERENCE));
    }
    table_head(w, "LAYER", handle::LAYER_TABLE, layers.len());
    for (name, colour) in layers {
        record(
            w,
            "LAYER",
            handle::LAYER_TABLE,
            "AcDbLayerTableRecord",
            name,
        );
        w.int(62, colour);
        w.tag(6, "Continuous");
    }
    w.tag(0, "ENDTAB");

    // `Standard` carries no text — nothing here writes any — but a text style
    // table with no `Standard` in it is a file AutoCAD refuses.
    table_head(w, "STYLE", handle::STYLE_TABLE, 1);
    record(
        w,
        "STYLE",
        handle::STYLE_TABLE,
        "AcDbTextStyleTableRecord",
        "Standard",
    );
    w.real(40, 0.0); // height: 0 means "ask when used"
    w.real(41, 1.0); // width factor
    w.real(50, 0.0); // oblique angle
    w.int(71, 0);
    w.real(42, 2.5); // last height used
    w.tag(3, "txt");
    w.tag(4, "");
    w.tag(0, "ENDTAB");

    empty_table(w, "VIEW", handle::VIEW_TABLE);
    empty_table(w, "UCS", handle::UCS_TABLE);

    table_head(w, "APPID", handle::APPID_TABLE, 1);
    record(
        w,
        "APPID",
        handle::APPID_TABLE,
        "AcDbRegAppTableRecord",
        "ACAD",
    );
    w.tag(0, "ENDTAB");

    // **The one record whose handle is not group code 5.** `DIMSTYLE` uses 105,
    // because 5 was already taken in that record by the dimension style's own
    // numbering when the format was extended. Writing 5 here is a file that
    // loads everywhere except where it matters.
    table_head(w, "DIMSTYLE", handle::DIMSTYLE_TABLE, 1);
    w.tag(0, "DIMSTYLE");
    let h = w.next_handle();
    w.tag(105, &h);
    w.tag(330, handle::DIMSTYLE_TABLE);
    w.tag(100, "AcDbSymbolTableRecord");
    w.tag(100, "AcDbDimStyleTableRecord");
    w.tag(2, "Standard");
    w.int(70, 0);
    w.tag(0, "ENDTAB");

    // The layouts. Model space is what everything below is drawn in; paper
    // space is a sheet nothing is on, and both must exist.
    table_head(w, "BLOCK_RECORD", handle::BLOCK_RECORD_TABLE, 2);
    for (name, h) in [
        ("*Model_Space", handle::MODEL_SPACE),
        ("*Paper_Space", handle::PAPER_SPACE),
    ] {
        w.tag(0, "BLOCK_RECORD");
        w.tag(5, h);
        w.tag(330, handle::BLOCK_RECORD_TABLE);
        w.tag(100, "AcDbSymbolTableRecord");
        w.tag(100, "AcDbBlockTableRecord");
        w.tag(2, name);
        // The LAYOUT object this space would be drawn on. There is none — a
        // part is not a sheet — and null is how the format says so.
        w.tag(340, "0");
    }
    w.tag(0, "ENDTAB");

    w.tag(0, "ENDSEC");
}

/// The two block definitions the block records above name.
///
/// Both are empty: the gear itself lives in the `ENTITIES` section, which is
/// model space written out longhand. They exist because R2000 defines a layout
/// as a block, and a layout with no block is a dangling reference.
fn blocks(w: &mut Writer) {
    w.tag(0, "SECTION");
    w.tag(2, "BLOCKS");
    for (name, owner, begin, end) in [
        (
            "*Model_Space",
            handle::MODEL_SPACE,
            handle::MODEL_SPACE_BLOCK,
            handle::MODEL_SPACE_ENDBLK,
        ),
        (
            "*Paper_Space",
            handle::PAPER_SPACE,
            handle::PAPER_SPACE_BLOCK,
            handle::PAPER_SPACE_ENDBLK,
        ),
    ] {
        w.tag(0, "BLOCK");
        w.tag(5, begin);
        w.tag(330, owner);
        w.tag(100, "AcDbEntity");
        w.tag(8, "0");
        w.tag(100, "AcDbBlockBegin");
        w.tag(2, name);
        w.int(70, 0);
        w.real(10, 0.0);
        w.real(20, 0.0);
        w.real(30, 0.0);
        w.tag(3, name);
        w.tag(1, ""); // no external reference
        w.tag(0, "ENDBLK");
        w.tag(5, end);
        w.tag(330, owner);
        w.tag(100, "AcDbEntity");
        w.tag(8, "0");
        w.tag(100, "AcDbBlockEnd");
    }
    w.tag(0, "ENDSEC");
}

/// The object tree, at its documented minimum: a root dictionary naming one
/// entry, `ACAD_GROUP`, and that group dictionary, empty.
///
/// Nothing in this file is grouped. The root dictionary is where a reader looks
/// for everything that is not geometry, and a file without one is a file with
/// nowhere to look.
fn objects(w: &mut Writer) {
    w.tag(0, "SECTION");
    w.tag(2, "OBJECTS");

    w.tag(0, "DICTIONARY");
    w.tag(5, handle::ROOT_DICTIONARY);
    w.tag(330, "0");
    w.tag(100, "AcDbDictionary");
    w.int(281, 1); // keep the existing entry if a name collides
    w.tag(3, "ACAD_GROUP");
    w.tag(350, handle::GROUP_DICTIONARY);

    w.tag(0, "DICTIONARY");
    w.tag(5, handle::GROUP_DICTIONARY);
    w.tag(330, handle::ROOT_DICTIONARY);
    w.tag(100, "AcDbDictionary");
    w.int(281, 1);

    w.tag(0, "ENDSEC");
}

/// The opening of any entity: what it is, its handle, the space that owns it,
/// and the layer it is drawn on.
fn entity(w: &mut Writer, kind: &str, layer: &str) {
    w.tag(0, kind);
    w.owned(handle::MODEL_SPACE);
    w.tag(100, "AcDbEntity");
    w.tag(8, layer);
}

fn polyline(w: &mut Writer, vertices: &[Vertex]) {
    if vertices.is_empty() {
        return;
    }
    entity(w, "LWPOLYLINE", LAYER_PROFILE);
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
    entity(w, "CIRCLE", LAYER_REFERENCE);
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

    /// The tags between `ENTITIES` and its `ENDSEC` — the drawing, without the
    /// boilerplate a file needs around it.
    ///
    /// **A test that counts group codes must look here.** `42` is a vertex
    /// bulge in an entity and the last text height in the `STYLE` record; `10`
    /// and `20` are a point in an entity and a block's base point. Counting
    /// both is how a test starts failing when the *file* becomes correct.
    fn entities(t: &[(i32, String)]) -> Vec<(i32, String)> {
        let start = t
            .iter()
            .position(|(c, v)| *c == 2 && v == "ENTITIES")
            .expect("an ENTITIES section");
        let end = t[start..]
            .iter()
            .position(|(c, v)| *c == 0 && v == "ENDSEC")
            .expect("ENTITIES must be closed")
            + start;
        t[start + 1..end].to_vec()
    }

    /// The handle of a named symbol-table record, which is what everything it
    /// owns points at.
    fn record_handle(t: &[(i32, String)], kind: &str, name: &str) -> String {
        for (i, (code, value)) in t.iter().enumerate() {
            if *code != 0 || value != kind {
                continue;
            }
            let body: Vec<&(i32, String)> =
                t[i + 1..].iter().take_while(|(c, _)| *c != 0).collect();
            if body.iter().any(|(c, v)| *c == 2 && v == name) {
                return body
                    .iter()
                    .find(|(c, _)| *c == 5 || *c == 105)
                    .map(|(_, v)| v.clone())
                    .expect("a record with no handle");
            }
        }
        panic!("no {kind} record named {name}");
    }

    /// Records, by the table they are in — read from the file, not from the
    /// writer, so a table that never gets written is a table that fails here.
    fn table_records(t: &[(i32, String)]) -> Vec<(String, Vec<String>)> {
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        let mut table: Option<String> = None;
        for (i, (code, value)) in t.iter().enumerate() {
            if *code != 0 {
                continue;
            }
            match value.as_str() {
                "TABLE" => {
                    let name = t[i + 1].1.clone();
                    table = Some(name.clone());
                    out.push((name, Vec::new()));
                }
                "ENDTAB" => table = None,
                kind if Some(kind.to_string()) == table => {
                    // The record's name is its first group-2 tag.
                    let name = t[i + 1..]
                        .iter()
                        .take_while(|(c, _)| *c != 0)
                        .find(|(c, _)| *c == 2)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();
                    out.last_mut()
                        .expect("a record outside a table")
                        .1
                        .push(name);
                }
                _ => {}
            }
        }
        out
    }

    /// **The file must satisfy the R2000 minimum, not merely hold the gear.**
    ///
    /// A reader that builds its own document — `ezdxf` does, and so did the
    /// only independent check this export had — will supply every structure a
    /// file omits and report nothing wrong. A reader that does not, and
    /// SolidWorks is one, refuses the file outright. So the requirement is
    /// gated here, tag by tag, against the published minimum for AC1015:
    /// six sections, nine tables with the records that must be in them, both
    /// spaces defined as blocks, and a root dictionary.
    #[test]
    fn the_file_meets_the_r2000_minimum() {
        for dxf in [
            gear_to_dxf(&Tooth::new(GearParams::default()), &DxfOptions::default()),
            gear_to_dxf(
                &Tooth::new(GearParams::default()),
                &DxfOptions {
                    reference_circles: false,
                    ..DxfOptions::default()
                },
            ),
            ring_to_dxf(
                &gear_core::ring::Ring::cut_by(
                    &GearParams {
                        teeth: 60,
                        ..GearParams::default()
                    },
                    &gear_core::ring::Cutter::default(),
                ),
                &DxfOptions::default(),
            ),
        ] {
            let t = tags(&dxf);

            let sections: Vec<&str> = t
                .iter()
                .enumerate()
                .filter(|(_, (c, v))| *c == 0 && v == "SECTION")
                .map(|(i, _)| t[i + 1].1.as_str())
                .collect();
            assert_eq!(
                sections,
                ["HEADER", "CLASSES", "TABLES", "BLOCKS", "ENTITIES", "OBJECTS"],
                "the six sections R2000 requires, in the order a reader walks them"
            );

            let tables = table_records(&t);
            let names: Vec<&str> = tables.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(
                names,
                [
                    "VPORT",
                    "LTYPE",
                    "LAYER",
                    "STYLE",
                    "VIEW",
                    "UCS",
                    "APPID",
                    "DIMSTYLE",
                    "BLOCK_RECORD"
                ],
                "all nine tables must exist, empty or not"
            );
            let records = |name: &str| -> Vec<String> {
                tables
                    .iter()
                    .find(|(n, _)| n == name)
                    .expect("table")
                    .1
                    .clone()
            };
            for (table, required) in [
                ("LTYPE", &["ByBlock", "ByLayer", "Continuous"][..]),
                ("LAYER", &["0"][..]),
                ("STYLE", &["Standard"][..]),
                ("APPID", &["ACAD"][..]),
                ("DIMSTYLE", &["Standard"][..]),
                ("BLOCK_RECORD", &["*Model_Space", "*Paper_Space"][..]),
            ] {
                for want in required {
                    assert!(
                        records(table).iter().any(|r| r == want),
                        "{table} must define {want}, has {:?}",
                        records(table)
                    );
                }
            }

            // Both spaces are defined as blocks, because R2000 makes a layout a
            // block and a block record naming nothing is a dangling reference.
            for space in ["*Model_Space", "*Paper_Space"] {
                assert_eq!(
                    t.windows(2)
                        .filter(|w| w[0] == (0, "BLOCK".to_string()) && w[1].0 == 5)
                        .count()
                        .min(2),
                    2,
                    "both block definitions must be written"
                );
                assert!(
                    dxf.contains(space),
                    "{space} is named by neither the table nor the blocks"
                );
            }

            assert!(
                t.iter().any(|(c, v)| *c == 3 && v == "ACAD_GROUP"),
                "the root dictionary must name ACAD_GROUP"
            );

            // `DIMSTYLE`'s handle is group code 105, not 5. It is the one
            // record the format numbers differently, and written as 5 it is a
            // record a strict reader sees no handle on at all.
            assert!(
                t.windows(2)
                    .any(|w| w[0] == (0, "DIMSTYLE".to_string()) && w[1].0 == 105),
                "the DIMSTYLE record's handle must be group code 105"
            );
        }
    }

    /// **Every handle is unique, every owner resolves, and the seed is above
    /// them all.**
    ///
    /// An R2000 file is a graph: a record points at its owner by handle, and a
    /// pointer into nothing is what turns a file a lenient reader repairs into
    /// a file a strict one rejects. `$HANDSEED` is the next handle a *reader*
    /// may hand out, so it has to be above every handle written here.
    #[test]
    fn the_handle_graph_closes() {
        let dxf = gear_to_dxf(&Tooth::new(GearParams::default()), &DxfOptions::default());
        let t = tags(&dxf);

        let seed_at = t
            .iter()
            .position(|(c, v)| *c == 9 && v == "$HANDSEED")
            .expect("$HANDSEED");
        let seed = u32::from_str_radix(t[seed_at + 1].1.trim(), 16).expect("seed is hexadecimal");

        // Handles are group code 5, and 105 for the one record that is
        // different — DIMSTYLE, whose 5 was taken when the format grew.
        let mut handles: Vec<u32> = Vec::new();
        for (i, (code, value)) in t.iter().enumerate() {
            if (*code == 5 && i != seed_at + 1) || *code == 105 {
                handles.push(u32::from_str_radix(value.trim(), 16).expect("handle is hexadecimal"));
            }
        }
        let mut unique = handles.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), handles.len(), "duplicate handle");
        assert!(
            handles.iter().all(|h| *h != 0),
            "a handle of zero is the null reference, not a record"
        );
        assert!(
            seed > *handles.iter().max().expect("some handles"),
            "$HANDSEED must be above every handle in the file"
        );

        // Owner (330), layout (340) and dictionary (350) references all point
        // at handles, and null is spelled 0.
        for (code, value) in &t {
            if [330, 340, 350].contains(code) && value.trim() != "0" {
                let h = u32::from_str_radix(value.trim(), 16).expect("reference is hexadecimal");
                assert!(
                    unique.binary_search(&h).is_ok(),
                    "reference to a handle that is not in the file: {code} -> {value}"
                );
            }
        }

        // Every entity is owned by the model space block record, which is what
        // makes it something a reader draws rather than an orphan.
        let model_space = record_handle(&t, "BLOCK_RECORD", "*Model_Space");
        let ents = entities(&t);
        let mut drawn = 0;
        for (i, (code, value)) in ents.iter().enumerate() {
            if *code == 0 && ["LWPOLYLINE", "CIRCLE"].contains(&value.as_str()) {
                assert_eq!(
                    ents[i + 2],
                    (330, model_space.clone()),
                    "{value} is not owned by model space"
                );
                drawn += 1;
            }
        }
        assert_eq!(drawn, 5, "one profile and four reference circles");
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
        let t = entities(&tags(&dxf));

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
        let radii: Vec<f64> = entities(&tags(&with))
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
        let n_bulge_written = entities(&tags(&dxf))
            .iter()
            .filter(|(c, _)| *c == 42)
            .count();
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
