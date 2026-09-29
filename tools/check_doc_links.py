#!/usr/bin/env python3
"""Every pointer into the documents resolves, and CLAUDE.md's checks are CI's.

**Pointers.** Every tracked file is read (`tools/sources.py`: `git ls-files`,
less the generated `web/src/wire`), and two forms of pointer are resolved:

  - `docs/<path>.md#<anchor>` as code and prose write it, relative to the
    repository root, with or without the anchor;
  - `[text](<target>)` as a document writes a link, relative to the file it is
    written in: `(state.md#the-canaries)`, `(#anchors)`, `(../README.md)`,
    `(history/audit.md#f12)`, `(../tools/check_all.sh)`.

The file a pointer names must exist, and an anchor must be a heading in it
(GitHub's slug; a repeated heading takes `-1`, `-2`, ...; a heading inside a
code fence is not one) or an `id` in an HTML file. The whole anchor is read, so
one with an `_` or a capital in it is not cut short and matched on its prefix.

A pointer to a heading survives an inserted section; a bare section number
does not, and one is reported as legacy outside the frozen `docs/history/` and
`handoff_inbound/`, whose numbers are its own document's.

Read are the text formats a pointer is written in (`TEXT`), and not the working
records `audit/` and `work/`: those cite the documents as they stood when a
finding or a plan was written, which is their point, and are edited in place
while the work they describe is in flight.

**The one list of CI.** CLAUDE.md's table of checks says, per command, whether
CI runs it. That column is held to `.github/workflows/ci.yml` as
`tools/ci_steps.py` reads it (the reader `tools/check_all.sh` runs from):

  - a row marked `yes` names commands a `tests` step runs;
  - a row marked `via nix flake check` names what one of flake.nix's checks runs,
    and every one of those is named;
  - a row marked `no` names commands no step runs;
  - every script, build and npm command a step runs has a `yes` row.

    tools/check_doc_links.py            # exits non-zero and names what is broken
    tools/check_doc_links.py --list     # what points where, for a restructure

Dependency-free, like its siblings here.
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ci_steps  # noqa: E402
import sources  # noqa: E402

ROOT = sources.ROOT

# `docs/state.md#the-canaries`, `docs/history/audit.md`, in prose or a comment.
# Not preceded by a path character, so `../docs/x.md` is read as a link below.
CODE = re.compile(r"(?<![\w./-])(docs/[\w./-]*?\.(?:md|html))(?:#([^)\s`'\"\]>,;|*]+))?")
# `[text](target)`; the target has no space.
MD_LINK = re.compile(r"\[[^\]\n]*\]\(([^)\s]+)\)")
LEGACY = re.compile(r"§\d[\d.]*")
FENCE = re.compile(r"^\s*(```|~~~)")
# The working records, dated by what they cite (see the docstring).
RECORDS = ("audit", "work")
# The formats a pointer is written in. Recorded output (`tools/golden/*.txt`,
# `*.json`) is what a command printed, not a pointer anyone wrote.
TEXT = {".md", ".rs", ".py", ".sh", ".ts", ".svelte", ".mjs", ".js", ".nix", ".yml", ".toml", ".html", ".css"}


def slug(text):
    """GitHub's anchor for a heading's text."""
    text = re.sub(r"`([^`]*)`", r"\1", text)  # code spans
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)  # links
    text = re.sub(r"<[^>]+>", "", text)  # inline html
    # Runs are not collapsed, because GitHub does not collapse them: an em dash
    # between two spaces anchors as `--`.
    return re.sub(r"[^\w\s-]", "", text.lower()).replace(" ", "-")


_anchor_cache = {}


def anchors(path):
    """Every anchor a file offers: its headings' slugs, or an HTML file's ids."""
    if path in _anchor_cache:
        return _anchor_cache[path]
    text = path.read_text(encoding="utf-8")
    out = set()
    if path.suffix == ".html":
        out = set(re.findall(r"\bid=\"([^\"]+)\"", text))
    else:
        seen = {}
        fenced = False
        for line in text.splitlines():
            if FENCE.match(line):
                fenced = not fenced
                continue
            m = None if fenced else re.match(r"^(#{1,6})\s+(.*?)\s*$", line)
            if not m:
                continue
            s = slug(m.group(2))
            n = seen.get(s, 0)
            seen[s] = n + 1
            out.add(s if n == 0 else f"{s}-{n}")
    _anchor_cache[path] = out
    return out


def is_path_target(target, markdown):
    """A link target that names a file or an anchor, not a URL or a Rust path.

    In Markdown every other target is a path, `(state#x)` included; in a doc
    comment, `[text](Foo)` is Rust's own link, so only a target shaped like a
    path is read."""
    if re.match(r"^[a-z][a-z0-9+.-]*:", target) or "::" in target:
        return False
    return markdown or bool(
        target.startswith(("#", "./", "../"))
        or "/" in target
        or re.search(r"\.[a-z]{2,4}(#|$)", target)
    )


def resolve(base, target, where):
    """(resolved, error) for one pointer: `target` read against `base`."""
    file, _, anchor = target.partition("#")
    path = (base / file).resolve() if file else where
    rel = str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    if not path.exists():
        return None, f"{rel} does not exist"
    if anchor and path.suffix in (".md", ".html"):
        have = anchors(path)
        if anchor not in have:
            near = ", ".join(sorted(a for a in have if a[:4] == anchor[:4])[:3])
            return None, f"{rel} has no #{anchor}  (alike: {near or 'none'})"
    return (f"{rel}#{anchor}" if anchor else rel), None


def pointers(broken, listing):
    used = set()
    legacy = {}
    for src in sources.tracked(TEXT):
        if src.relative_to(ROOT).parts[0] in RECORDS:
            continue
        try:
            text = src.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        rel = src.relative_to(ROOT)
        markdown = src.suffix == ".md"
        fenced = False
        for n, line in enumerate(text.splitlines(), 1):
            if markdown and FENCE.match(line):
                fenced = not fenced
            if fenced:
                continue
            found = []
            rest = line
            for m in MD_LINK.finditer(line):
                target = m.group(1)
                if not is_path_target(target, markdown) or (target.startswith("#") and not markdown):
                    continue
                found.append((src.parent, target))
                rest = rest.replace(m.group(0), " ")
            for m in CODE.finditer(rest):
                found.append((ROOT, m.group(1) + (f"#{m.group(2)}" if m.group(2) else "")))
            # A sentence's own full stop or colon after a pointer is not part of
            # the anchor; no slug contains either.
            found = [(b, re.sub(r"(#.*?)[.:]+$", r"\1", t)) for b, t in found]
            for base, target in found:
                ok, error = resolve(base, target, src)
                if error:
                    broken.append(f"{rel}:{n}  ({target}) {error}")
                else:
                    used.add(ok)
                    if listing:
                        print(f"{rel}:{n} -> {ok}")
            # The frozen record, and the prior work whose numbers are its own.
            if "history" in rel.parts or rel.parts[0] == "handoff_inbound":
                continue
            for hit in LEGACY.findall(line):
                legacy.setdefault(hit, []).append(f"{rel}:{n}")
    return used, legacy


# --- the one list of CI ---------------------------------------------------

# What a CI step invokes that a check row must name.
INVOKED = re.compile(r"(tools/[\w./-]+|nix build \S+|nix flake check|npm run \w+|npm test)")


def claude_table():
    """(line, commands, in-CI) per row of CLAUDE.md's table of checks."""
    lines = (ROOT / "CLAUDE.md").read_text(encoding="utf-8").splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith("| Run | Catches | In CI |"))
    rows, ci = [], None
    for i in range(start + 2, len(lines)):
        line = lines[i]
        if not line.startswith("|"):
            break
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if cells[-1] != '"':
            ci = cells[-1]
        commands = [re.sub(r"\s*<[^>]*>", "", c).strip() for c in re.findall(r"`([^`]+)`", cells[0])]
        rows.append((i + 1, commands, ci))
    return rows


def ci_table_law():
    steps, errors = ci_steps.read_steps(("deploy",))
    standins, more = ci_steps.read_flake()
    errors = [f"ci.yml / flake.nix: {e}" for e in errors + more]
    runs = [" ".join(s["run"].split()) for s in steps if "run" in s]
    named, via = set(), set()
    for line, commands, ci in claude_table():
        where = f"CLAUDE.md:{line}"
        if ci == "yes":
            kind = "yes"
        elif ci and "nix flake check" in ci:
            kind = "flake"
        elif ci and ci.startswith("no"):
            kind = "no"
        else:
            errors.append(f"{where}: In CI reads {ci!r}; say yes, via `nix flake check`, or no")
            continue
        for cmd in commands:
            parts = [" ".join(p.split()) for p in cmd.split("&&")]
            in_ci = any(all(p in r for p in parts) for r in runs)
            if kind == "yes":
                named.add(cmd)
                if not in_ci:
                    errors.append(f"{where}: `{cmd}` is marked yes and no ci.yml step runs it")
            elif kind == "flake":
                via.add(cmd)
                if cmd not in standins:
                    errors.append(
                        f"{where}: `{cmd}` is marked via nix flake check, which runs none such"
                        f" (it runs: {'; '.join(standins)})"
                    )
            elif in_ci:
                errors.append(f"{where}: `{cmd}` is marked no and a ci.yml step runs it")
    for s in standins:
        if s not in via:
            errors.append(f"CLAUDE.md: flake.nix runs `{s}` and no row marked via nix flake check names it")
    for r in runs:
        for hit in INVOKED.findall(r):
            if hit == "nix flake check":
                continue
            if not any(hit in c for c in named):
                errors.append(f"CLAUDE.md: ci.yml runs `{hit}` and no row marked yes names it")
    return sorted(set(errors))


def main():
    listing = "--list" in sys.argv
    broken = []
    used, legacy = pointers(broken, listing)
    table = ci_table_law()

    if broken:
        print("Pointers that do not resolve:\n", file=sys.stderr)
        for b in broken:
            print("  " + b, file=sys.stderr)
    if table:
        print("\nCLAUDE.md's table of checks and ci.yml disagree:\n", file=sys.stderr)
        for e in table:
            print("  " + e, file=sys.stderr)
    if legacy:
        total = sum(len(v) for v in legacy.values())
        print(
            f"\n{total} legacy section-number references, across "
            f"{len(legacy)} sections, with nothing stable to resolve against:\n",
            file=sys.stderr,
        )
        for section, where in sorted(legacy.items(), key=lambda kv: -len(kv[1])):
            print(f"  {section:>10}  ×{len(where):<4} {where[0]}", file=sys.stderr)
        print("\nRewrite each as docs/<file>.md#<anchor>, which survives a reorder.", file=sys.stderr)
    if broken or table or legacy:
        return 1

    print(f"{len(used)} distinct pointers, all resolving; CLAUDE.md's checks are ci.yml's")
    return 0


if __name__ == "__main__":
    sys.exit(main())
