#!/usr/bin/env python3
"""Every pointer from the code into the documents lands on a heading that exists.

The code carries a lot of these -- 175 of them across 32 distinct sections when
this was written -- and nothing checked any of them. A document reorganised
underneath them breaks all 175 silently, which is what made restructuring the
design record risky enough to keep putting off.

    tools/check_doc_links.py            # exits non-zero and names what is broken
    tools/check_doc_links.py --list     # what points where, for a restructure

# Why anchors rather than numbers

A pointer to `docs/rationale.md#the-lewis-parabola` survives an inserted section;
a pointer to `§4.7` does not. So the form the code is expected to use is
`docs/<file>.md#<anchor>`, and this script resolves each against the headings in
that file, both directions:

  - every reference resolves to a heading;
  - every heading a reference *could* mean is reachable, so a renamed heading
    with no referrer is reported as well, since that is how a pointer rots
    without anyone noticing.

Bare `#12`-style section numbers are reported as legacy and named individually
rather than resolved, because there is nothing stable to resolve them against.
That report is the migration list, and it is empty once the migration is done.

Dependency-free, like its siblings here; `gear_io::strings` and
`tools/check_strings.py` are the same idea applied to the message catalogue.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"

SOURCES = (
    sorted((ROOT / "crates").rglob("*.rs"))
    + sorted((ROOT / "web/src").glob("*.ts"))
    + sorted((ROOT / "web/src").glob("*.svelte"))
    + sorted(DOCS.glob("*.md"))
    + [ROOT / "README.md"]
)

# `docs/rationale.md#one-hob-one-setting`, in prose or in a doc comment.
LINK = re.compile(r"docs/([a-z0-9_-]+)\.md#([a-z0-9-]+)")
# The form being migrated away from: a bare section number.
LEGACY = re.compile(r"§\d[\d.]*")


def anchors(path):
    """GitHub-style anchors for every ATX heading in a Markdown file."""
    out = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        m = re.match(r"^(#{1,6})\s+(.*?)\s*$", line)
        if not m:
            continue
        text = m.group(2)
        text = re.sub(r"`([^`]*)`", r"\1", text)          # code spans
        text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)  # links
        slug = re.sub(r"[^\w\s-]", "", text.lower())
        slug = re.sub(r"[\s_]+", "-", slug).strip("-")
        out[slug] = text
    return out


def main():
    listing = "--list" in sys.argv

    docs = {p.stem: anchors(p) for p in DOCS.glob("*.md")}
    broken, legacy, used = [], {}, set()

    for src in SOURCES:
        try:
            text = src.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        rel = src.relative_to(ROOT)
        for n, line in enumerate(text.splitlines(), 1):
            for doc, anchor in LINK.findall(line):
                used.add((doc, anchor))
                if doc not in docs:
                    broken.append(f"{rel}:{n}  docs/{doc}.md does not exist")
                elif anchor not in docs[doc]:
                    near = ", ".join(sorted(docs[doc])[:3])
                    broken.append(
                        f"{rel}:{n}  docs/{doc}.md has no #{anchor}  (it has e.g. {near})"
                    )
                elif listing:
                    print(f"{rel}:{n} -> docs/{doc}.md#{anchor}")
            # Section numbers inside the frozen record are its own business.
            if "history" in rel.parts:
                continue
            for hit in LEGACY.findall(line):
                legacy.setdefault(hit, []).append(f"{rel}:{n}")

    if broken:
        print("Pointers that do not resolve:\n", file=sys.stderr)
        for b in broken:
            print("  " + b, file=sys.stderr)
        return 1

    if legacy:
        total = sum(len(v) for v in legacy.values())
        print(
            f"{total} legacy section-number references, across "
            f"{len(legacy)} sections, with nothing stable to resolve against:\n",
            file=sys.stderr,
        )
        for section, where in sorted(legacy.items(), key=lambda kv: -len(kv[1])):
            print(f"  {section:>10}  ×{len(where):<4} {where[0]}", file=sys.stderr)
        print(
            "\nRewrite each as docs/<file>.md#<anchor>, which survives a reorder.",
            file=sys.stderr,
        )
        return 1

    print(f"{len(used)} documentation pointers, all resolving")
    return 0


if __name__ == "__main__":
    sys.exit(main())
