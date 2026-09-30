//! The differential oracle's records (`tests/data/field_oracle/`, its README), as each step's test
//! reads them: every record's inputs, the prototype's outputs, and the tolerance it states; and
//! the check that the copy is the prototype's version 2, byte for byte.

use serde_json::Value;

/// One record: `{id, fn, inputs, outputs, tol}`.
pub(crate) struct Record<'a> {
    pub id: &'a str,
    pub inputs: &'a Value,
    pub outputs: &'a Value,
    pub tol: &'a Value,
}

/// A module's records, in file order.
pub(crate) fn records(module: &Value) -> Vec<Record<'_>> {
    module["records"]
        .as_array()
        .expect("a module file holds a list of records")
        .iter()
        .map(|r| Record {
            id: r["id"].as_str().expect("every record has an id"),
            inputs: &r["inputs"],
            outputs: &r["outputs"],
            tol: &r["tol"],
        })
        .collect()
}

/// A number the record must hold.
pub(crate) fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("a number, found {v}"))
}

/// A number the record may hold: `null` is the prototype's `None`.
pub(crate) fn opt(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| num(v))
}

/// The closed-form rule's bound on a value: `rel` of its size, and `rel` absolute where the value
/// passes zero (the README's tolerance rule), so a value that is zero on one piece is not held to
/// a relative bound of nothing.
pub(crate) fn closed_bound(rel: f64, oracle: f64, passes_zero: bool) -> f64 {
    if passes_zero {
        rel * oracle.abs().max(1.0)
    } else {
        rel * oracle.abs()
    }
}

/// How far the port misses the oracle, over the bound: at most 1 reproduces it.
pub(crate) fn miss(port: f64, oracle: f64, bound: f64) -> f64 {
    let d = (port - oracle).abs();
    if d == 0.0 {
        0.0
    } else {
        d / bound
    }
}

/// The worst miss seen, where, how many values were compared, and how many of them missed (a
/// ratio above 1, or not a number): a fault ten times the tolerance is caught by every value it
/// touches missing, not by the worst of them.
#[derive(Default)]
pub(crate) struct Worst {
    pub ratio: f64,
    pub at: String,
    pub count: usize,
    pub missed: usize,
}

impl Worst {
    pub fn see(&mut self, ratio: f64, at: impl FnOnce() -> String) {
        self.count += 1;
        if ratio > 1.0 || ratio.is_nan() {
            self.missed += 1;
        }
        if ratio > self.ratio || ratio.is_nan() {
            self.ratio = ratio;
            self.at = at();
        }
    }
}

/// The version of the oracle this copy must be: `index.json`'s `version`.
pub(crate) const VERSION: u64 = 2;

/// What `index` and the module files disagree on: the version, and per module the record ids
/// the index lists against those the file holds (as sets: the index lists them sorted). Empty
/// when the copy is whole and of [`VERSION`].
pub(crate) fn index_mismatches(index: &Value, files: &[(&str, &Value)]) -> Vec<String> {
    let mut out = Vec::new();
    if index["version"].as_u64() != Some(VERSION) {
        out.push(format!("version {} is not {VERSION}", index["version"]));
    }
    let listed = index["modules"].as_object().expect("index lists modules");
    if listed.len() != files.len() {
        out.push(format!(
            "{} modules listed, {} files",
            listed.len(),
            files.len()
        ));
    }
    for (name, file) in files {
        let ids = |v: &Value| -> Vec<String> {
            let mut ids: Vec<String> = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().expect("an id").to_owned())
                        .collect()
                })
                .unwrap_or_default();
            ids.sort();
            ids
        };
        let held: Vec<String> = {
            let mut v: Vec<String> = records(file).iter().map(|r| r.id.to_owned()).collect();
            v.sort();
            v
        };
        if file["module"].as_str() != Some(name) {
            out.push(format!("{name}.json names module {}", file["module"]));
        }
        if ids(&listed[*name]) != held {
            out.push(format!("{name}: the index's records are not the file's"));
        }
    }
    out
}

/// RFC 1321's MD5 of `bytes`, as the 32 hex digits `md5sum` prints: the sums the README's
/// provenance table lists.
pub(crate) fn md5_hex(bytes: &[u8]) -> String {
    // RFC 1321, 3.4: each round's four shifts.
    const SHIFTS: [[u32; 4]; 4] = [
        [7, 12, 17, 22],
        [5, 9, 14, 20],
        [4, 11, 16, 23],
        [6, 10, 15, 21],
    ];
    // RFC 1321, 3.3: the buffer's initial words A, B, C, D.
    const START: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    // RFC 1321, 3.2: a message is 512-bit blocks; 64 bits of each last block hold its length.
    const BLOCK: usize = 64;
    const LENGTH: usize = 8;

    let sines: Vec<u32> = (1..=64).map(md5_sine).collect();
    // RFC 1321, 3.1–3.2: a 1 bit, zeros up to a block's end less its length, then the length in
    // bits, modulo 2⁶⁴. After the 1 bit the message is n + 1 bytes long, so the zeros are
    // (BLOCK − LENGTH − 1 − n) mod BLOCK, written without a negative.
    let zeros = (2 * BLOCK - LENGTH - 1 - bytes.len() % BLOCK) % BLOCK;
    let bits = u64::try_from(bytes.len())
        .expect("a length fits 64 bits")
        .wrapping_mul(8);
    let mut message = bytes.to_vec();
    message.push(0x80);
    message.resize(message.len() + zeros, 0);
    message.extend_from_slice(&bits.to_le_bytes());
    debug_assert_eq!(message.len() % BLOCK, 0);

    let mut sum = START;
    for block in message.chunks_exact(BLOCK) {
        let words: Vec<u32> = block
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let [mut a, mut b, mut c, mut d] = sum;
        for (step, sine) in sines.iter().enumerate() {
            // RFC 1321, 3.4: the round's function F, G, H or I, and the word it reads.
            let (mixed, word) = match step / 16 {
                0 => ((b & c) | (!b & d), step),
                1 => ((b & d) | (c & !d), (5 * step + 1) % 16),
                2 => (b ^ c ^ d, (3 * step + 5) % 16),
                _ => (c ^ (b | !d), (7 * step) % 16),
            };
            let turned = a
                .wrapping_add(mixed)
                .wrapping_add(*sine)
                .wrapping_add(words[word])
                .rotate_left(SHIFTS[step / 16][step % 4]);
            (a, b, c, d) = (d, b.wrapping_add(turned), b, c);
        }
        for (s, x) in sum.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(x);
        }
    }
    sum.iter()
        .flat_map(|w| w.to_le_bytes())
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// RFC 1321, 3.4: `T[i] = ⌊2³² |sin i|⌋`, `i` in radians.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "⌊2³² |sin i|⌋ is an integer in [0, 2³²)"
)]
fn md5_sine(i: u32) -> u32 {
    let two_32 = f64::from(u32::MAX) + 1.0;
    (f64::from(i).sin().abs() * two_32).floor() as u32
}

/// The README's provenance table: each file it lists and its md5, in its order. A row is
/// `` | `file` | `md5` | `` in the provenance section, up to the next section; a row of that shape
/// anywhere else is not a sum.
pub(crate) fn provenance(readme: &str) -> Vec<(&str, &str)> {
    readme
        .lines()
        .skip_while(|l| !l.starts_with("## Provenance"))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .filter(|l| l.starts_with("| `"))
        .filter_map(|l| match l.split('`').collect::<Vec<_>>()[..] {
            [_, file, _, sum, _] => Some((file, sum)),
            _ => None,
        })
        .collect()
}

/// What the copy's bytes and the README's provenance table disagree on: a file whose md5 is not
/// the one sum the table lists for it (a stale file, one listed twice or not at all), a row for
/// a file the check does not read, and a file in the copy's directory it does not read. `files`
/// is what the tests read, `listing` every file the directory holds; the README is the one file
/// the table cannot list. Empty when every file is the prototype's, byte for byte.
pub(crate) fn provenance_mismatches(
    readme: &str,
    files: &[(&str, &[u8])],
    listing: &[String],
) -> Vec<String> {
    let table = provenance(readme);
    let read = |name: &str| files.iter().any(|(f, _)| *f == name);
    let mut out = Vec::new();
    for (name, bytes) in files {
        let sum = md5_hex(bytes);
        let listed: Vec<&str> = table
            .iter()
            .filter(|(f, _)| f == name)
            .map(|(_, s)| *s)
            .collect();
        if listed != [sum.as_str()] {
            out.push(format!("{name}: md5 {sum}, the README lists {listed:?}"));
        }
    }
    for (name, _) in &table {
        if !read(name) {
            out.push(format!(
                "{name}: the README lists it, the check does not read it"
            ));
        }
    }
    for name in listing {
        if name != "README.md" && !read(name) {
            out.push(format!("{name}: in the copy, the check does not read it"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The copy's directory.
    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/field_oracle");

    /// The README, whose provenance table lists every other file's md5.
    const README: &str = include_str!("../../tests/data/field_oracle/README.md");

    /// Every other file of the copy, as the tests read it.
    const FILES: [(&str, &[u8]); 10] = [
        (
            "index.json",
            include_bytes!("../../tests/data/field_oracle/index.json"),
        ),
        (
            "form.json",
            include_bytes!("../../tests/data/field_oracle/form.json"),
        ),
        (
            "kernel.json",
            include_bytes!("../../tests/data/field_oracle/kernel.json"),
        ),
        (
            "across.json",
            include_bytes!("../../tests/data/field_oracle/across.json"),
        ),
        (
            "compliance.json",
            include_bytes!("../../tests/data/field_oracle/compliance.json"),
        ),
        (
            "gap.json",
            include_bytes!("../../tests/data/field_oracle/gap.json"),
        ),
        (
            "matched.json",
            include_bytes!("../../tests/data/field_oracle/matched.json"),
        ),
        (
            "state.json",
            include_bytes!("../../tests/data/field_oracle/state.json"),
        ),
        (
            "phase.json",
            include_bytes!("../../tests/data/field_oracle/phase.json"),
        ),
        (
            "make_oracle.py",
            include_bytes!("../../tests/data/field_oracle/make_oracle.py"),
        ),
    ];

    /// The modules `index.json` lists, each `<module>.json`.
    const MODULES: [&str; 8] = [
        "form",
        "kernel",
        "across",
        "compliance",
        "gap",
        "matched",
        "state",
        "phase",
    ];

    fn bytes(name: &str) -> &'static [u8] {
        FILES
            .iter()
            .find(|(f, _)| *f == name)
            .map(|(_, b)| *b)
            .unwrap_or_else(|| panic!("{name} is one of the copy's files"))
    }

    fn parse(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).expect("the oracle's JSON parses")
    }

    fn copy() -> (Value, Vec<(&'static str, Value)>) {
        let files = MODULES
            .iter()
            .map(|m| (*m, parse(bytes(&format!("{m}.json")))))
            .collect();
        (parse(bytes("index.json")), files)
    }

    fn mismatches(index: &Value, files: &[(&'static str, Value)]) -> Vec<String> {
        let refs: Vec<(&str, &Value)> = files.iter().map(|(n, v)| (*n, v)).collect();
        index_mismatches(index, &refs)
    }

    /// Every file in the copy's directory, by name.
    fn listing() -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(DIR)
            .expect("the copy's directory reads")
            .map(|e| {
                e.expect("an entry reads")
                    .file_name()
                    .into_string()
                    .expect("a name in UTF-8")
            })
            .collect();
        names.sort();
        names
    }

    /// The repository's copy is version 2, whole and byte for byte the prototype's: every module
    /// the index lists is here and holds exactly the records the index names, 106 in all, the
    /// index records no failure, and each of the 10 files other than the README has the md5 the
    /// README's provenance table lists for it, in a directory that holds nothing else.
    #[test]
    fn the_copy_is_version_two_and_whole() {
        let (index, files) = copy();
        assert_eq!(mismatches(&index, &files), Vec::<String>::new());
        let total: usize = files.iter().map(|(_, f)| records(f).len()).sum();
        assert_eq!((files.len(), total), (8, 106));
        assert_eq!(index["errors"].as_array().map(Vec::len), Some(0));

        assert_eq!(
            provenance_mismatches(README, &FILES, &listing()),
            Vec::<String>::new()
        );
        assert_eq!((provenance(README).len(), listing().len()), (10, 11));
    }

    /// The version check's plants: version 1's index (no `version`), an index a record short, and
    /// a file holding one record more than the index (a record added to the copy by hand) are
    /// each refused.
    #[test]
    fn the_version_check_fails_its_plants() {
        let (index, files) = copy();
        let mut v1 = index.clone();
        v1.as_object_mut().expect("an object").remove("version");
        assert_eq!(mismatches(&v1, &files).len(), 1);

        let mut short = index.clone();
        short["modules"]["form"]
            .as_array_mut()
            .expect("a list")
            .pop();
        assert_eq!(mismatches(&short, &files).len(), 1);

        let mut more = files.clone();
        let extra = more[0].1["records"][0].clone();
        more[0].1["records"]
            .as_array_mut()
            .expect("a list")
            .push(extra);
        assert_eq!(mismatches(&index, &more).len(), 1);
    }

    /// The copy with `name`'s bytes replaced.
    fn with(name: &str, planted: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
        FILES
            .iter()
            .map(|(f, b)| {
                (
                    *f,
                    if *f == name {
                        planted.to_vec()
                    } else {
                        b.to_vec()
                    },
                )
            })
            .collect()
    }

    fn refs<'a>(files: &'a [(&'static str, Vec<u8>)]) -> Vec<(&'static str, &'a [u8])> {
        files.iter().map(|(f, b)| (*f, b.as_slice())).collect()
    }

    /// Whether the version check (the index's version and its record ids) passes a copy.
    fn ids_pass(files: &[(&'static str, Vec<u8>)]) -> bool {
        let parsed: Vec<(&str, Value)> = files
            .iter()
            .filter_map(|(f, b)| f.strip_suffix(".json").map(|m| (m, parse(b))))
            .collect();
        let index = &parsed
            .iter()
            .find(|(m, _)| *m == "index")
            .expect("an index")
            .1;
        let modules: Vec<(&str, &Value)> = parsed
            .iter()
            .filter(|(m, _)| *m != "index")
            .map(|(m, v)| (*m, v))
            .collect();
        index_mismatches(index, &modules).is_empty()
    }

    /// The provenance check's plants, each refused as one mismatch naming its file. The first two
    /// are the near misses the version check passes (same version, same record ids): version 2's
    /// index beside a `kernel.json` in version 1's shape (its panel record without the `scale`
    /// and `route` version 2 added: the separated panels' routes), and `state.json` with one
    /// digit of one value changed. Then: a file in the directory the check does not read (a
    /// version 1 file left beside the copy), the table without a file's row, the table listing a
    /// file twice (the second time with version 1's sum), and the check not reading a file the
    /// table lists. And one that passes: a row of the table's shape outside its section, before
    /// it and after it, is not read as a sum.
    #[test]
    fn the_provenance_check_fails_its_plants() {
        let listing = listing();
        let refused = |files: &[(&str, &[u8])], readme: &str, listing: &[String], name: &str| {
            let m = provenance_mismatches(readme, files, listing);
            assert_eq!(m.len(), 1, "{name}: {m:?}");
            assert!(m[0].starts_with(name), "{name}: {m:?}");
        };

        let kernel = std::str::from_utf8(bytes("kernel.json")).expect("UTF-8");
        let (from, to) = (
            kernel
                .find("\n    \"scale\": [")
                .expect("the panel's scale"),
            kernel
                .find("\n    \"n_width\": [")
                .expect("the panel's n_width"),
        );
        let v1_shaped = format!("{}{}", &kernel[..from], &kernel[to..]);
        assert!(v1_shaped.contains("\"panel\"") && !v1_shaped.contains("\"route\""));
        let stale = with("kernel.json", v1_shaped.as_bytes());
        assert!(ids_pass(&stale), "a near miss: the version check passes it");
        refused(&refs(&stale), README, &listing, "kernel.json");

        let state = bytes("state.json");
        let at = state
            .windows(12)
            .position(|w| w == b"\"outputs\": {")
            .expect("state's first outputs");
        let digit = at
            + state[at..]
                .iter()
                .position(u8::is_ascii_digit)
                .expect("a digit");
        let mut changed = state.to_vec();
        changed[digit] = if changed[digit] == b'9' {
            b'8'
        } else {
            changed[digit] + 1
        };
        let one_digit = with("state.json", &changed);
        assert!(
            ids_pass(&one_digit),
            "a near miss: the version check passes it"
        );
        refused(&refs(&one_digit), README, &listing, "state.json");

        let whole = FILES.to_vec();
        let mut stray = listing.clone();
        stray.push("kernel_v1.json".to_owned());
        refused(&whole, README, &stray, "kernel_v1.json");

        let row = "| `kernel.json` | `35b0e86116762ee4ae94c9bbf65f8ccc` |\n";
        assert!(README.contains(row));
        refused(&whole, &README.replace(row, ""), &listing, "kernel.json");
        let twice = README.replace(
            row,
            &format!("{row}| `kernel.json` | `1f3b72ba69630cb1309a7bb468f2acb7` |\n"),
        );
        refused(&whole, &twice, &listing, "kernel.json");
        let stale_row = "| `kernel.json` | `1f3b72ba69630cb1309a7bb468f2acb7` |\n";
        let outside = format!(
            "{}## Later\n\n{stale_row}",
            README.replacen("## Files", &format!("{stale_row}\n## Files"), 1)
        );
        assert_eq!(outside.matches(stale_row).count(), 2);
        assert_eq!(
            provenance_mismatches(&outside, &whole, &listing),
            Vec::<String>::new()
        );

        let unread: Vec<(&str, &[u8])> = FILES
            .iter()
            .filter(|(f, _)| *f != "make_oracle.py")
            .copied()
            .collect();
        let rest: Vec<String> = listing
            .iter()
            .filter(|f| *f != "make_oracle.py")
            .cloned()
            .collect();
        refused(&unread, README, &rest, "make_oracle.py");
    }

    /// Every number literal of a JSON text, in document order. A string is skipped whole, escapes
    /// included, so a digit in an id or a note is not read as a number.
    fn number_literals(text: &str) -> Vec<&str> {
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    i += 1;
                    while bytes[i] != b'"' {
                        i += if bytes[i] == b'\\' { 2 } else { 1 };
                    }
                    i += 1;
                }
                b'-' | b'0'..=b'9' => {
                    let start = i;
                    while i < bytes.len()
                        && matches!(bytes[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                    {
                        i += 1;
                    }
                    out.push(&text[start..i]);
                }
                _ => i += 1,
            }
        }
        out
    }

    /// The text's own value of a literal: `str::parse`, correctly rounded, as Python's `repr`
    /// wrote each float to round-trip.
    fn written(literal: &str) -> f64 {
        literal.parse().expect("a JSON number is a Rust float")
    }

    /// The literals of `text` that `read` reads to other bits than [`written`]'s.
    fn misread(text: &str, read: impl Fn(&str) -> f64) -> Vec<&str> {
        number_literals(text)
            .into_iter()
            .filter(|t| read(t).to_bits() != written(t).to_bits())
            .collect()
    }

    /// The bits of every number `value` holds, as the harness reads it ([`num`]), sorted.
    fn held(value: &Value) -> Vec<u64> {
        fn walk(v: &Value, out: &mut Vec<u64>) {
            match v {
                Value::Number(_) => out.push(num(v).to_bits()),
                Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
                Value::Object(o) => o.values().for_each(|x| walk(x, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(value, &mut out);
        out.sort_unstable();
        out
    }

    /// How many numbers the parsed `value` holds that `text`'s literals, as [`written`], do not
    /// (as multisets of bits): 0 when the harness reads every number as it was written.
    fn held_apart(value: &Value, text: &str) -> usize {
        let mut wrote: Vec<u64> = number_literals(text)
            .into_iter()
            .map(|t| written(t).to_bits())
            .collect();
        wrote.sort_unstable();
        let (held, mut apart, mut j) = (held(value), 0, 0);
        for bits in held {
            while j < wrote.len() && wrote[j] < bits {
                j += 1;
            }
            if j < wrote.len() && wrote[j] == bits {
                j += 1;
            } else {
                apart += 1;
            }
        }
        apart
    }

    /// serde_json's default float reader (without `float_roundtrip`), step for step: the digits
    /// as a `u64` significand (those past its overflow dropped, an integer's counted in the
    /// exponent), then `significand as f64` times or over `10^|e|`, each of the two rounded: the
    /// double rounding that misreads a 17-digit literal.
    fn default_reader(literal: &str) -> f64 {
        let (negative, rest) = literal
            .strip_prefix('-')
            .map_or((false, literal), |r| (true, r));
        let (mantissa, e) = rest.split_once(['e', 'E']).map_or((rest, 0), |(m, e)| {
            (m, e.parse::<i32>().expect("an exponent"))
        });
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let (mut significand, mut exponent) = (0_u64, e);
        for c in whole.bytes() {
            match significand
                .checked_mul(10)
                .and_then(|s| s.checked_add(u64::from(c - b'0')))
            {
                Some(s) => significand = s,
                None => exponent += 1,
            }
        }
        for c in fraction.bytes() {
            match significand
                .checked_mul(10)
                .and_then(|s| s.checked_add(u64::from(c - b'0')))
            {
                Some(s) => (significand, exponent) = (s, exponent - 1),
                None => break,
            }
        }
        let mut f = significand as f64;
        while exponent < -308 && f != 0.0 {
            (f, exponent) = (f / 1e308, exponent + 308);
        }
        // serde_json's `POW10[|e|]`: the literal `1e|e|`, correctly rounded.
        let pow: f64 = format!("1e{}", exponent.abs())
            .parse()
            .expect("a power of ten");
        f = if exponent >= 0 { f * pow } else { f / pow };
        if negative {
            -f
        } else {
            f
        }
    }

    /// Every number of every JSON file of the copy reads to the bits its text was written as:
    /// the parse each step's test reads (serde_json with `float_roundtrip`) holds, as a multiset,
    /// exactly the literals' own values, and reads each literal alone to them. The multiset alone
    /// is not enough: under the default reader one of `gap.json`'s 646 misreads lands on another
    /// literal's value, and it counts 645. Plants, each refused by exactly the literals it
    /// touches: serde_json's default reader, which rounds twice and misreads 46 of
    /// `kernel.json`'s literals (2496 of the copy's); two near misses a comparison by value passes, a reader one ulp
    /// off on a literal written once and one that drops the sign of `gap.json`'s one `-0.0`;
    /// and a parse that holds one number an ulp off.
    #[test]
    fn every_number_reads_to_the_bits_it_was_written_as() {
        let serde = |t: &str| serde_json::from_str::<f64>(t).expect("a number");
        let text = |name: &str| std::str::from_utf8(bytes(name)).expect("UTF-8");
        let json: Vec<&str> = FILES
            .iter()
            .map(|(f, _)| *f)
            .filter(|f| f.ends_with(".json"))
            .collect();
        let mut literals = 0;
        for name in &json {
            let (text, value) = (text(name), parse(bytes(name)));
            assert_eq!(held(&value).len(), number_literals(text).len(), "{name}");
            assert_eq!(held_apart(&value, text), 0, "{name}");
            assert_eq!(misread(text, serde), Vec::<&str>::new(), "{name}");
            literals += number_literals(text).len();
        }
        assert_eq!((json.len(), literals), (9, 25_041));

        // What serde_json without `float_roundtrip` misread in each file, measured before the
        // feature was set: the plant reproduces it file for file.
        let default: Vec<usize> = json
            .iter()
            .map(|name| misread(text(name), default_reader).len())
            .collect();
        assert_eq!(default, [0, 150, 46, 206, 46, 646, 39, 1309, 54]);

        let kernel = text("kernel.json");
        let all = number_literals(kernel);
        let once = *all
            .iter()
            .find(|t| all.iter().filter(|u| u == t).count() == 1)
            .expect("a literal written once");
        let ulp_off = |t: &str| {
            let v = written(t);
            if t == once {
                f64::from_bits(v.to_bits() + 1)
            } else {
                v
            }
        };
        assert_eq!(misread(kernel, ulp_off), vec![once]);

        let gap = text("gap.json");
        let unsigned = |t: &str| {
            let v = written(t);
            if v == 0.0 {
                0.0
            } else {
                v
            }
        };
        assert_eq!(misread(gap, unsigned), vec!["-0.0"]);

        let mut nudged = parse(bytes("kernel.json"));
        let r = &mut nudged["records"][0]["inputs"]["r"][1];
        *r = Value::from(f64::from_bits(num(r).to_bits() + 1));
        assert_eq!(held_apart(&nudged, kernel), 1);
    }

    /// [`closed_bound`], to the bit, at literal values: `rel` of the value's size, floored at `rel`
    /// absolute only where the value passes zero. A step's 10× law cannot pin this: its fault is
    /// ten times the larger reading, so a bound up to ten times too loose on either branch passes
    /// it. Plants, each holding the one pin it cannot fail: the bound loosened 100× (only zero
    /// room at zero holds), and the relative branch floored at 1 as the other is, 4× looser on
    /// a value of 0.25 (every pin above 1 in size or passing zero holds).
    #[test]
    fn the_closed_bound_is_the_readmes_rule() {
        let pins = |bound: fn(f64, f64, bool) -> f64| {
            [
                (0.5, true, 1e-13),
                (-0.25, true, 1e-13),
                (0.0, true, 1e-13),
                (20.0, true, 2e-12),
                (-20.0, false, 2e-12),
                (0.25, false, 2.5e-14),
                (0.0, false, 0.0),
            ]
            .into_iter()
            .filter(|&(x, passes_zero, b)| bound(1e-13, x, passes_zero) == b)
            .count()
        };
        assert_eq!(pins(closed_bound), 7);
        assert_eq!(pins(|rel, x, z| 100.0 * closed_bound(rel, x, z)), 1);
        assert_eq!(pins(|rel, x, _| rel * x.abs().max(1.0)), 5);
    }

    /// The md5 is RFC 1321's: its test suite (appendix A.5), whose lengths 0 … 80 bytes cover
    /// one block, a length that spills into a second block (62), and two whole blocks (80).
    #[test]
    fn the_md5_reproduces_rfc_1321s_suite() {
        let suite = [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ];
        for (text, sum) in suite {
            assert_eq!(md5_hex(text.as_bytes()), sum, "{text:?}");
        }
    }
}
