#!/usr/bin/env python3
"""**Absence is typed.** A number standing for "there is none" is refused in
production code: `unwrap_or(<literal>)`, `map_or(<literal>, …)`, the same
through a closure (`unwrap_or_else(|| <literal>)`, `map_or_else(|| <literal>,
…)`), `f64::INFINITY`, `NEG_INFINITY` or `NAN` used at all, and an array of
literals returned (`return (p, [1.0, 0.0])`, a direction standing for none)
— unless the line says why, with a comment `// absence: <why>`.

Each of these has put a wrong number in a report (pattern C of the Stage 1
exit review, work/stage1-exit.md): an `unwrap_or(0.0)` read as a length,
`past = ∞` driving an addendum to −2.4985, efficiency 0.00 for a flow that
was refused. An `Option` says the same thing and cannot be added to.

    tools/check_absence.py              # exits non-zero on a new site
    tools/check_absence.py --found      # every site, in the list's format
    tools/check_absence.py --self-test  # the planted faults

The sites that were there when this gate came are listed in
`tools/allow_absence.txt`, each with the stage that empties it; `gear-core`'s
must be gone by Stage 2's exit. The list is keyed by file, function and site,
with a count, and must match **exactly**, so a new site fails and so does a
listed one that is gone.

A literal is made of numbers (`0.`, `1e-9`, `-1`) and numeric constants
(`f64::NAN`, `usize::MAX` — an integer's end is a sentinel as much as a
float's infinity), signed, grouped into tuples or arrays, or combined by
arithmetic (`1e-3 / 4.0`). A closure counts where it returns one: `|| 0.0`,
`|_| 0.0`, `|| { 0.0 }`. A reason excuses the one site on its line; a line
with two sites and one reason excuses neither. A boolean is not read:
`map_or(false, …)` answers "is there one that …?", and absence answers that
truthfully. `unwrap_or_default()` is not read either: which default it gives
depends on a type this text does not know. Read as `tools/rust_source.py`
reads production code: comments and strings blanked, test code left out.
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rust_source  # noqa: E402

ROOT = rust_source.ROOT
ALLOW = ROOT / "tools" / "allow_absence.txt"
STAGE = re.compile(r"^Stage \d")
WHY = re.compile(r"//\s*absence:\s*\S")

NUMBER = r"\d[\d_]*(?:\.(?:\d[\d_]*)?)?(?:[eE][+-]?\d[\d_]*)?(?:_?[fiu](?:8|16|32|64|128|size))?"
NUMERIC_TYPE = r"(?:f32|f64|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)"
CONSTANT = rf"(?:(?:std|core)\s*::\s*)?<?\s*{NUMERIC_TYPE}\s*>?\s*::\s*[A-Z_]+"
ATOM = re.compile(rf"{CONSTANT}|{NUMBER}")
# What may sit between atoms in a literal: signs, arithmetic, grouping.
GLUE = re.compile(r"^[\s()\[\],+\-*/]*$")
# A closure's body: a block, or an expression; a return type needs a block.
CLOSURE = re.compile(
    r"^\s*(?:move\s+)?\|[^|]*\|\s*"
    r"(?:(?:->\s*[^{]+?\s*)?\{\s*(?P<block>[^{}]*?)\s*\}|(?P<expr>[^{].*?))\s*$",
    re.S,
)
CALL = re.compile(r"\.\s*(unwrap_or|map_or|unwrap_or_else|map_or_else)\s*\(")
# A `return` whose value holds an array of literals — `return (p, [1.0, 0.0])`
# — is a fallback standing for a direction, a point or a vector that is not
# there: an early exit with made-up coordinates (strength.rs's line of action
# at the base circle returned `[1, 0]`).
RETURN = re.compile(r"(?<![\w.])return\b")
LITERAL_ARRAY = re.compile(r"\[([^\[\]]*,[^\[\]]*)\]")
# Any use of the three, however it is reached: `f64::NAN`, `std::f64::NAN`,
# `<f64>::NAN`, or a bare `NAN` brought in by a `use`.
SENTINEL = re.compile(r"(?<![\w.])(INFINITY|NEG_INFINITY|NAN)\b")


def is_literal(text):
    """Whether `text` is made of numbers and numeric constants alone —
    signed, grouped into tuples or arrays, or combined by arithmetic."""
    atoms = ATOM.findall(text)
    return bool(atoms) and bool(GLUE.match(ATOM.sub(" ", text)))


def first_argument(code, open_paren):
    """The text of the argument after `open_paren`, to its `,` or `)`."""
    depth = 0
    for k in range(open_paren + 1, len(code)):
        ch = code[k]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            if depth == 0:
                return code[open_paren + 1:k]
            depth -= 1
        elif ch == "," and depth == 0:
            return code[open_paren + 1:k]
    return code[open_paren + 1:]


def sites(sources):
    """`(file, function, site, line)` of every number standing for none on
    a line that does not say why."""
    out = []
    for s in sources:
        found, spans = [], []
        for m in CALL.finditer(s.code):
            arg = first_argument(s.code, m.end() - 1)
            if m.group(1).endswith("_else"):
                closure = CLOSURE.match(arg)
                body = closure and (closure.group("block") if closure.group("block") is not None else closure.group("expr"))
                if not body or not is_literal(body):
                    continue
            else:
                if not is_literal(arg):
                    continue
                body = arg
            found.append((m.start(), f"{m.group(1)}({' '.join(body.split())})"))
            spans.append((m.end(), m.end() + len(arg)))
        for m in RETURN.finditer(s.code):
            end = s.code.find(";", m.end())
            value = s.code[m.end():end if end >= 0 else len(s.code)]
            for a in LITERAL_ARRAY.finditer(value):
                if is_literal(a.group(1)):
                    found.append((m.start(), f"return[{' '.join(a.group(1).split())}]"))
                    spans.append((m.end(), m.end() + len(value)))
        # A constant is one site with the call it is the default of.
        for m in SENTINEL.finditer(s.code):
            if not any(a <= m.start() < b for a, b in spans):
                found.append((m.start(), m.group(1)))
        found = [(at, site) for at, site in found if not s.in_test(at)]
        per_line = {}
        for at, _ in found:
            per_line[s.line(at)] = per_line.get(s.line(at), 0) + 1
        for at, site in found:
            line = s.line(at)
            # A reason excuses the one site on its line; a line with two has
            # one reason for two absences, and excuses neither.
            if per_line[line] == 1 and WHY.search(s.comments.get(line, "")):
                continue
            out.append((s.rel, s.function(at), site.replace(" ", ""), line))
    return out


def counted(found):
    keys = {}
    for f, fn, site, _ in found:
        keys[(f, fn, site)] = keys.get((f, fn, site), 0) + 1
    return keys


def read_allow(path):
    """`({(file, fn, site): (count, stage)}, faults)`."""
    allow, bad = {}, []
    for n, line in enumerate(path.read_text().splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) != 5 or not parts[3].isdigit() or not STAGE.match(parts[4]):
            bad.append(f"{path.name}:{n}: want `file<TAB>fn<TAB>site<TAB>count<TAB>Stage …`: {line!r}")
            continue
        allow[tuple(parts[:3])] = (int(parts[3]), parts[4])
    return allow, bad


def compare(found, allow):
    bad = []
    have = counted(found)
    for key, n in sorted(have.items()):
        listed = allow.get(key, (0, ""))[0]
        if n > listed:
            lines = [str(l) for f, fn, site, l in found if (f, fn, site) == key]
            bad.append(f"{key[0]}:{','.join(lines)} in {key[1]}: {key[2]} stands for none "
                       f"({n} here, {listed} listed); return an Option or a named refusal, "
                       f"or say `// absence: <why>` on the line")
    for key, (n, stage) in sorted(allow.items()):
        if have.get(key, 0) < n:
            bad.append(f"{ALLOW.name}: {' '.join(key)} lists {n}, {have.get(key, 0)} found; take it off the list")
    return bad


def main_check():
    found = sites(rust_source.production())
    allow, bad = read_allow(ALLOW)
    bad += compare(found, allow)
    for b in bad:
        print(b, file=sys.stderr)
    if bad:
        return 1
    by_stage = {}
    for (f, _, _), (n, stage) in allow.items():
        crate = f.split("/")[1]
        by_stage.setdefault((crate, stage), 0)
        by_stage[(crate, stage)] += n
    listed = "; ".join(f"{c} {n} to {st}" for (c, st), n in sorted(by_stage.items()))
    print(f"no new number stands for none; {len(found)} listed ({listed})")
    return 0


def found_listing():
    for (f, fn, site), n in sorted(counted(sites(rust_source.production())).items()):
        print(f"{f}\t{fn}\t{site}\t{n}\tStage ?")
    return 0


# ------------------------------------------------------------ self-test ---

FIXTURE = r'''
/// A doc comment's `unwrap_or(0.0)` and `f64::NAN` are prose.
fn flagged(a: Option<f64>, b: Option<(f64, f64)>) -> f64 {
    let s = "unwrap_or(0.0) in a string, f64::INFINITY too";
    // unwrap_or(0.0) in a comment
    let x = a.unwrap_or(0.0);
    let y = a.map_or(1.0, |v| v * 2.0);
    let z = a.unwrap_or(-1);
    let w = a
        .unwrap_or(
            1e-9,
        );
    let p = b.unwrap_or((0.0, 0.0));
    let q = a.unwrap_or_else(|| 0.0);
    let r = a.map_or_else(|| f64::MAX, |v| v);
    let t = a.unwrap_or(std::f64::INFINITY);
    let big = f64::NEG_INFINITY;
    let nan = core::f64::NAN;
    let n = NAN;
    x + y
}
fn fine(a: Option<f64>, lim: f64) -> f64 {
    let x = a.unwrap_or(lim);
    let y = a.map_or(false, |v| v > 0.0);
    let z = a.unwrap_or_else(|| lim * 2.0);
    let u = a.unwrap_or(0.0); // absence: the fixture says why
    let v = f64::INFINITY; // absence: an empty sweep's identity for min
    let d = a.unwrap_or_default();
    x.is_nan() || x.is_infinite()
}
fn why_elsewhere(a: Option<f64>) -> f64 {
    // absence: said on the line above, which is not the line
    a.unwrap_or(0.0)
}
fn empty_why(a: Option<f64>) -> f64 {
    a.unwrap_or(0.0) // absence:
}
#[cfg(test)]
mod tests {
    fn inside(a: Option<f64>) -> f64 { a.unwrap_or(0.0) + f64::NAN }
}
fn after_the_tests(a: Option<f64>) -> f64 { a.unwrap_or(0.0) }
fn fallback(len: f64, p: [f64; 2], d: [f64; 2]) -> ([f64; 2], [f64; 2]) {
    if len < f64::MIN_POSITIVE {
        return (p, [1.0, 0.0]);
    }
    if len > 1.0 {
        return (p, [d[0], -d[1]]);
    }
    if len > 2.0 {
        return (p, [1.0, 0.0]); // absence: the fixture says why
    }
    (p, [1.0, 0.0])
}
fn near_misses(a: Option<usize>, b: Option<f64>, c: Result<f64, ()>) -> f64 {
    let i = a.unwrap_or(usize::MAX);
    let j = a.map_or(i32::MIN, |v| v as i32);
    let k = b.unwrap_or(0.);
    let l = c.unwrap_or_else(|_| 0.0);
    let n = b.unwrap_or_else(|| { 0.0 });
    let o = c.map_or_else(|_| -1.0, |v| v);
    let p = b.unwrap_or(1e-3 / 4.0);
    let q = b.unwrap_or(0.0) + b.unwrap_or(f64::NAN); // absence: one reason, two sites
    let r = a.unwrap_or(u32::try_from(7).unwrap_or(0) as usize);
    let s = b.unwrap_or_else(|| -> f64 { 0.0 });
    let t = <f64>::INFINITY;
    let u = b.unwrap_or(<f64>::NAN);
    k
}
'''
WANT = sorted([
    ("flagged", "unwrap_or(0.0)"), ("flagged", "map_or(1.0)"), ("flagged", "unwrap_or(-1)"),
    ("flagged", "unwrap_or(1e-9)"), ("flagged", "unwrap_or((0.0,0.0))"),
    ("flagged", "unwrap_or_else(0.0)"), ("flagged", "map_or_else(f64::MAX)"),
    ("flagged", "unwrap_or(std::f64::INFINITY)"),
    ("flagged", "NEG_INFINITY"), ("flagged", "NAN"), ("flagged", "NAN"),
    ("why_elsewhere", "unwrap_or(0.0)"), ("empty_why", "unwrap_or(0.0)"),
    ("after_the_tests", "unwrap_or(0.0)"),
    ("near_misses", "unwrap_or(usize::MAX)"), ("near_misses", "map_or(i32::MIN)"),
    ("near_misses", "unwrap_or(0.)"), ("near_misses", "unwrap_or_else(0.0)"),
    ("near_misses", "unwrap_or_else(0.0)"), ("near_misses", "map_or_else(-1.0)"),
    ("near_misses", "unwrap_or(1e-3/4.0)"), ("near_misses", "unwrap_or(0.0)"),
    ("near_misses", "unwrap_or(f64::NAN)"), ("near_misses", "unwrap_or(0)"),
    ("near_misses", "unwrap_or_else(0.0)"), ("near_misses", "INFINITY"),
    ("near_misses", "unwrap_or(<f64>::NAN)"),
    ("fallback", "return[1.0,0.0]"),
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
        found = sites(rust_source.production(root=root))
    got = sorted((fn, site) for _, fn, site, _ in found)
    expect(f"the fixture's {len(WANT)} sites are found, and nothing else", got == WANT)
    if got != WANT:
        print(f"       found {got}")
    for fn, site, why in (
        ("flagged", "unwrap_or_else(0.0)", "a literal returned through a closure"),
        ("flagged", "map_or_else(f64::MAX)", "a constant returned through a closure"),
        ("flagged", "unwrap_or(1e-9)", "a call split over lines"),
        ("flagged", "unwrap_or((0.0,0.0))", "a tuple of literals"),
        ("flagged", "unwrap_or(std::f64::INFINITY)", "a path-qualified constant"),
        ("why_elsewhere", "unwrap_or(0.0)", "a reason on the line above"),
        ("empty_why", "unwrap_or(0.0)", "an empty reason"),
        ("after_the_tests", "unwrap_or(0.0)", "production code after a test module"),
        ("near_misses", "unwrap_or(usize::MAX)", "an integer's end as the default"),
        ("near_misses", "map_or(i32::MIN)", "an integer's other end"),
        ("near_misses", "unwrap_or(0.)", "a float written `0.`"),
        ("near_misses", "unwrap_or(1e-3/4.0)", "literal arithmetic"),
        ("near_misses", "map_or_else(-1.0)", "a closure over the error, `|_|`"),
        ("near_misses", "unwrap_or(f64::NAN)", "two sites on a line with one reason"),
        ("near_misses", "unwrap_or(0)", "a site inside another's argument"),
        ("near_misses", "INFINITY", "a constant reached as `<f64>::`"),
        ("near_misses", "unwrap_or(<f64>::NAN)", "a default reached as `<f64>::`"),
        ("fallback", "return[1.0,0.0]", "an array of literals returned early"),
    ):
        expect(f"found: {why}", (fn, site) in got)
    expect("not found: a variable, a boolean, a computed closure, a reason, the default, a predicate",
           all(fn != "fine" for fn, _ in got))
    expect("found: `|| { 0.0 }`, `|_| 0.0` and `|| -> f64 { 0.0 }`",
           sum(1 for fn, site in got if (fn, site) == ("near_misses", "unwrap_or_else(0.0)")) == 3)
    expect("not found: test code or prose", all(fn not in ("inside",) for fn, _ in got))
    expect("found once: a returned array of literals; not: a computed one, a reason, a tail value",
           sum(1 for fn, _ in got if fn == "fallback") == 1)
    listed = {k: (n, "Stage 2 (Q3)") for k, n in counted(found).items()}
    expect("every site listed passes", compare(found, listed) == [])
    fewer = dict(listed)
    fewer.pop(("crates/gear-core/src/lib.rs", "after_the_tests", "unwrap_or(0.0)"))
    expect("a site not listed fails", len(compare(found, fewer)) == 1)
    gone = {**listed, ("crates/gear-core/src/lib.rs", "gone", "NAN"): (1, "Stage 2 (Q3)")}
    expect("a listed site that is gone fails", len(compare(found, gone)) == 1)
    expect("a second site in a listed function fails", len(compare(found + [found[0]], listed)) == 1)
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as f:
        f.write("crates/a.rs\tfn\tNAN\t1\tlater\n")
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
        sys.exit("usage: check_absence.py [--found | --self-test]")
    return main_check()


if __name__ == "__main__":
    sys.exit(main())
