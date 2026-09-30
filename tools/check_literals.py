#!/usr/bin/env python3
"""**A tolerance has a name.** A float literal below 1e-3 in `gear-core`'s
production code is a tolerance, a floor or a nudge, and it has to be a named
`const` — where its basis can be stated once and a reader can find every use.

The recurring fault it guards is a tolerance set by feel: an idle `1e-9`, a
`1e-12` guard, a `1e-4` margin, each written where it was needed and argued
nowhere (pattern B of the Stage 1 exit review, work/stage1-exit.md). A
literal inside a `const` item's initializer is named; one anywhere else
fails, unless it is listed in `tools/allow_literals.txt` with the stage that
empties the list.

    tools/check_literals.py              # exits non-zero on an unnamed one
    tools/check_literals.py --found      # every site, in the list's format
    tools/check_literals.py --self-test  # the planted faults

Read: every float literal (a decimal point, an exponent or an `f32`/`f64`
suffix; underscores allowed) in production code, as `tools/rust_source.py`
reads it — comments and strings blanked, `#[cfg(test)]` items and test-only
module files left out. Exactly zero is not a tolerance and is not read. So is
**a run of literals joined by `*` or `/`**, `1e-3 / 4.0` or `x / 2.0 /
1000.0`, which is one constant however it is spelt: its value is taken left
to right, inverted where the run follows a `/`, and it is read where no
literal in it is small on its own. A lone divisor (`x / 1000.0`) is a unit
and is not read.

The list is keyed by file, function and literal, with a count, and must match
what is found **exactly**: a new site fails, and so does a listed one that is
gone, so the list only shrinks by being edited.
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_source  # noqa: E402

ROOT = rust_source.ROOT
ALLOW = ROOT / "tools" / "allow_literals.txt"
CRATES = ("gear-core",)
BELOW = 1e-3

FLOAT = re.compile(
    r"(?<![\w.])(\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?(?:_?f(?:32|64))?)(?![\w.])"
)
# A `const` item: its name, then everything to its `;`.
CONST = re.compile(r"\bconst\s+([A-Z_][A-Z0-9_]*)\s*:")
STAGE = re.compile(r"^Stage \d")


def is_float(text):
    t = text.replace("_", "")
    return "." in t or "e" in t.lower() or t.endswith(("f32", "f64"))


def value(text):
    return float(re.sub(r"_?f(32|64)$", "", text.replace("_", "")))


def named_spans(code):
    """`(start, end)` of every `const` item's initializer: a `const NAME:`
    whose type is followed by `=`. A const generic parameter, `<const N:
    usize>`, has a `,` or `>` there instead, and names nothing."""
    out = []
    for m in CONST.finditer(code):
        depth = 0
        for k in range(m.end(), len(code)):
            ch = code[k]
            if ch in "([<":
                depth += 1
            elif ch in ")]>":
                depth -= 1
                if depth < 0:
                    break
            elif depth == 0 and ch in "=,;{":
                break
        if code[k] == "=":
            out.append((k, rust_source.item_end(code, k)))
    return out


# A run of numeric literals joined by `*` or `/`: one constant, however it is
# spelt.
LITERAL = r"\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?(?:_?f(?:32|64))?"
RUN = re.compile(rf"(?<![\w.])(?P<pre>/\s*)?(?P<run>{LITERAL}(?:\s*[*/]\s*{LITERAL})+)(?![\w.])")


def run_value(run, divided):
    """What a run multiplies by, evaluated left to right as Rust does, and
    inverted where the run itself follows a `/`."""
    parts = re.split(r"\s*([*/])\s*", run)
    ops = ["/" if divided else "*"] + parts[1::2]
    v = 1.0
    for op, lit in zip(ops, parts[0::2]):
        x = value(lit)
        if x == 0 and op == "/":
            return None
        v = v * x if op == "*" else v / x
    return v


def sites(sources):
    """`(file, function, literal, line)` of every unnamed small literal, and
    of every run of literals whose product falls small with none of its own
    literals small."""
    out = []
    for s in sources:
        named = named_spans(s.code)
        unnamed = lambda at: not s.in_test(at) and not any(a <= at < b for a, b in named)
        for m in FLOAT.finditer(s.code):
            text = m.group(1)
            if not is_float(text):
                continue
            v = value(text)
            if v == 0 or v >= BELOW:
                continue
            if unnamed(m.start()):
                out.append((s.rel, s.function(m.start()), text, s.line(m.start())))
        for m in RUN.finditer(s.code):
            run = m.group("run")
            lits = re.split(r"\s*[*/]\s*", run)
            if not any(is_float(x) for x in lits) or any(0 < value(x) < BELOW for x in lits):
                continue
            v = run_value(run, bool(m.group("pre")))
            if v is None or v == 0 or abs(v) >= BELOW:
                continue
            at = m.start("run")
            if unnamed(at):
                text = ("/ " if m.group("pre") else "") + re.sub(r"\s*([*/])\s*", r" \1 ", run)
                out.append((s.rel, s.function(at), text, s.line(at)))
    return out


def counted(found):
    keys = {}
    for f, fn, lit, _ in found:
        keys[(f, fn, lit)] = keys.get((f, fn, lit), 0) + 1
    return keys


def read_allow(path):
    """`({(file, fn, literal): (count, stage)}, faults)`."""
    allow, bad = {}, []
    for n, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != 5 or not parts[3].isdigit() or not STAGE.match(parts[4]):
            bad.append(f"{path.name}:{n}: want `file<TAB>fn<TAB>literal<TAB>count<TAB>Stage …`: {line!r}")
            continue
        allow[tuple(parts[:3])] = (int(parts[3]), parts[4])
    return allow, bad


def compare(found, allow):
    """Faults: a site not listed, or listed more or fewer times than found."""
    bad = []
    have = counted(found)
    for key, n in sorted(have.items()):
        listed = allow.get(key, (0, ""))[0]
        if n > listed:
            lines = [str(l) for f, fn, lit, l in found if (f, fn, lit) == key]
            bad.append(f"{key[0]}:{','.join(lines)} in {key[1]}: {key[2]} is unnamed ({n} here, {listed} listed); "
                       f"name it as a `const` with its basis")
    for key, (n, stage) in sorted(allow.items()):
        if have.get(key, 0) < n:
            bad.append(f"{ALLOW.name}: {' '.join(key)} lists {n}, {have.get(key, 0)} found; take it off the list")
    return bad


def main_check():
    found = sites(rust_source.production(CRATES))
    allow, bad = read_allow(ALLOW)
    bad += compare(found, allow)
    for b in bad:
        print(b, file=sys.stderr)
    if bad:
        return 1
    stages = sorted({stage for _, stage in allow.values()})
    print(f"no new unnamed float literal below {BELOW:g} in {', '.join(CRATES)}; "
          f"{len(found)} listed, to be named by {', '.join(stages)}")
    return 0


def found_listing():
    found = sites(rust_source.production(CRATES))
    for (f, fn, lit), n in sorted(counted(found).items()):
        print(f"{f}\t{fn}\t{lit}\t{n}\tStage ?")
    return 0


# ------------------------------------------------------------ self-test ---

# A production file: every site the gate must find, and every one it must not.
FIXTURE = r'''
/// A doc comment's 1e-9 is prose.
const NAMED: f64 = 1e-9;
pub(crate) const ALSO_NAMED: f64 =
    4.0 * 1e-12;
const TABLE: Pair = Pair { a: 2e-5, b: [1e-7, 0.5] };
fn flagged(x: f64) -> bool {
    let s = "1e-9 in a string";
    let c = 'e';
    // 1e-9 in a comment
    /* 1e-9 in a block /* nested 1e-9 */ comment */
    x < 1e-9 || x < 0.0001 || x < 1.0e-4 || x < 5E-7 || x < 1e-9_f64 || x < 0.000_1
        || x > -3e-4 && x != 0.0 && x < 1e-3 && x < 2e-3 && x < 1e3 && x < 1e-3f64
}
fn lifetimes<'a>(x: &'a f64) -> f64 { *x * 1e-6 }
const fn not_a_const_item() -> f64 { 1e-8 }
fn looks_named() { let TOL = 1e-10; }
#[cfg(test)]
mod tests {
    fn inside() { let _ = 1e-12; }
}
fn after_the_tests() { let _ = 1e-11; }
fn with_array(pair: &dyn Fn([f64; 2]) -> f64) -> f64 { 1e-5 }
fn generic<const N: usize>(x: f64) -> f64 { let y = x * 1e-7; y }
fn arithmetic(x: f64) -> f64 { x * 1e-3/4.0 + x * 0.5 * 0.001 + x / 2.0 / 1000.0 + x / 1000.0 + 2.0 * 3.0 + 1e3 * 1e-4 }
const RANGE: Range<f64> = 3e-6..1.0;
#[cfg(test)]
fn test_only() { let _ = 1e-12; }
'''
# Every unnamed small literal of the fixture, by function.
WANT = sorted([
    ("flagged", "1e-9"), ("flagged", "0.0001"), ("flagged", "1.0e-4"), ("flagged", "5E-7"),
    ("flagged", "1e-9_f64"), ("flagged", "0.000_1"), ("flagged", "3e-4"),
    ("lifetimes", "1e-6"), ("not_a_const_item", "1e-8"), ("looks_named", "1e-10"),
    ("after_the_tests", "1e-11"), ("with_array", "1e-5"), ("generic", "1e-7"),
    ("arithmetic", "1e-3 / 4.0"), ("arithmetic", "0.5 * 0.001"), ("arithmetic", "/ 2.0 / 1000.0"),
    ("arithmetic", "1e-4"),
])


def self_test():
    import tempfile

    results = []

    def expect(name, ok):
        results.append(ok)
        print(f"{'ok   ' if ok else 'WRONG'}  {name}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        src = root / "crates" / "gear-core" / "src"
        src.mkdir(parents=True)
        (src / "lib.rs").write_text(FIXTURE)
        found = sites(rust_source.production(CRATES, root))
    got = sorted((fn, lit) for _, fn, lit, _ in found)
    expect(f"the fixture's {len(WANT)} unnamed literals are found, and nothing else", got == WANT)
    if got != WANT:
        print(f"       found {got}")
    # The near misses, each a gate that stops too early or reads too much:
    for fn, lit, why in (
        ("after_the_tests", "1e-11", "production code after a test module"),
        ("flagged", "0.000_1", "a literal with an underscore"),
        ("flagged", "5E-7", "an upper-case exponent"),
        ("flagged", "1e-9_f64", "a suffixed literal"),
        ("looks_named", "1e-10", "a `let` with a constant's name"),
        ("not_a_const_item", "1e-8", "a `const fn` body"),
        ("with_array", "1e-5", "a function whose signature holds `[f64; 2]`, keyed by its name"),
        ("generic", "1e-7", "a function with a const generic parameter, which names nothing"),
        ("arithmetic", "1e-3 / 4.0", "a quotient of literals none of which is small"),
        ("arithmetic", "0.5 * 0.001", "a product of literals at and above 1e-3"),
        ("arithmetic", "/ 2.0 / 1000.0", "a run that divides, after a `/`"),
    ):
        expect(f"found: {why}", (fn, lit) in got)
    for lit, why in (("1e-12", "test code"), ("1e-3", "1e-3 itself"), ("2e-5", "a const struct literal"),
                     ("3e-6", "a const whose type has angle brackets"), ("0.0", "zero")):
        expect(f"not found: {why}", all(l != lit for _, l in got))
    # The list: exact both ways, and each entry names its stage.
    key = ("crates/gear-core/src/lib.rs", "after_the_tests", "1e-11")
    listed = {k: (1, "Stage 3 (N)") for k in counted(found)}
    expect("every site listed passes", compare(found, listed) == [])
    fewer = dict(listed)
    del fewer[key]
    expect("a site not listed fails", len(compare(found, fewer)) == 1)
    extra = {**listed, ("crates/gear-core/src/lib.rs", "gone", "1e-9"): (1, "Stage 3 (N)")}
    expect("a listed site that is gone fails", len(compare(found, extra)) == 1)
    twice = found + [found[-1]]
    expect("a second site of a listed literal in the same function fails", len(compare(twice, listed)) == 1)
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write("crates/a.rs\tfn\t1e-9\t1\tsomeday\n")
    _, bad = read_allow(Path(f.name))
    Path(f.name).unlink()
    expect("an entry that names no stage is refused", len(bad) == 1)
    wrong = results.count(False)
    print("every plant behaves" if not wrong else f"{wrong} plant(s) misbehave")
    return 1 if wrong else 0


def main():
    args = sys.argv[1:]
    if args == ["--self-test"]:
        return self_test()
    if args == ["--found"]:
        return found_listing()
    if args:
        sys.exit("usage: check_literals.py [--found | --self-test]")
    return main_check()


if __name__ == "__main__":
    sys.exit(main())
