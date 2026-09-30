#!/usr/bin/env python3
"""**Absence is typed.** A number standing for "there is none" is refused in
production code: `unwrap_or(<literal>)`, `map_or(<literal>, …)`, the same
through a closure (`unwrap_or_else(|| <literal>)`, `map_or_else(|| <literal>,
…)`), and `f64::INFINITY`, `NEG_INFINITY` or `NAN` used at all — unless the
line says why, with a comment `// absence: <why>`.

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

A literal is a number, an `f32`/`f64` constant, or a tuple or array of them.
A boolean is not read: `map_or(false, …)` answers "is there one that …?", and
absence answers that truthfully. `unwrap_or_default()` is not read either:
which default it gives depends on a type this text does not know. Read as
`tools/rust_source.py` reads production code: comments and strings blanked,
test code left out.
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

NUMBER = r"-?\s*\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?(?:_?[fiu](?:8|16|32|64|128|size))?"
CONSTANT = r"-?\s*(?:(?:std|core)\s*::\s*)?f(?:32|64)\s*::\s*[A-Z_]+"
ATOM = rf"(?:{NUMBER}|{CONSTANT})"
# A literal, or a tuple or array of them.
LITERAL = re.compile(rf"^\s*(?:{ATOM}|[(\[]\s*{ATOM}(?:\s*,\s*{ATOM})*\s*,?\s*[)\]])\s*$")
CALL = re.compile(r"\.\s*(unwrap_or|map_or|unwrap_or_else|map_or_else)\s*\(")
SENTINEL = re.compile(r"(?<![\w:])(?:(?:std|core)\s*::\s*)?(?:f(?:32|64)\s*::\s*)?(INFINITY|NEG_INFINITY|NAN)\b")


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
                closure = re.match(r"\s*\|\s*\|\s*(.*)$", arg, re.S)
                if not closure or not LITERAL.match(closure.group(1)):
                    continue
                body = closure.group(1)
            else:
                if not LITERAL.match(arg):
                    continue
                body = arg
            found.append((m.start(), f"{m.group(1)}({' '.join(body.split())})"))
            spans.append((m.end(), m.end() + len(arg)))
        # A constant is one site with the call it is the default of.
        for m in SENTINEL.finditer(s.code):
            if not any(a <= m.start() < b for a, b in spans):
                found.append((m.start(), m.group(1)))
        for at, site in found:
            if s.in_test(at):
                continue
            line = s.line(at)
            if WHY.search(s.comments.get(line, "")):
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
'''
WANT = sorted([
    ("flagged", "unwrap_or(0.0)"), ("flagged", "map_or(1.0)"), ("flagged", "unwrap_or(-1)"),
    ("flagged", "unwrap_or(1e-9)"), ("flagged", "unwrap_or((0.0,0.0))"),
    ("flagged", "unwrap_or_else(0.0)"), ("flagged", "map_or_else(f64::MAX)"),
    ("flagged", "unwrap_or(std::f64::INFINITY)"),
    ("flagged", "NEG_INFINITY"), ("flagged", "NAN"), ("flagged", "NAN"),
    ("why_elsewhere", "unwrap_or(0.0)"), ("empty_why", "unwrap_or(0.0)"),
    ("after_the_tests", "unwrap_or(0.0)"),
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
    ):
        expect(f"found: {why}", (fn, site) in got)
    expect("not found: a variable, a boolean, a computed closure, a reason, the default, a predicate",
           all(fn != "fine" for fn, _ in got))
    expect("not found: test code or prose", all(fn not in ("inside",) for fn, _ in got))
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
